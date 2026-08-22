//! Deterministic mesh-based hidden-line classification (Task-177).

use std::collections::BTreeMap;

use opencad_geometry::MeshSet;

use crate::projection::ProjectionKind;

/// Depth tolerance used when comparing projected triangles, in meters.
pub const HIDDEN_LINE_DEPTH_TOLERANCE_M: f64 = 1.0e-7;
/// Projection-space tolerance used for degenerate points and 2D intersections,
/// in meters.
pub const HIDDEN_LINE_PROJECTION_TOLERANCE_M: f64 = 1.0e-9;
/// Parameter tolerance used when splitting a projected edge, dimensionless.
pub const HIDDEN_LINE_PARAMETER_TOLERANCE: f64 = 1.0e-9;
/// Barycentric inclusion tolerance, dimensionless.
pub const HIDDEN_LINE_BARYCENTRIC_TOLERANCE: f64 = 1.0e-9;

/// Visibility of a projected drawing edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineVisibility {
    /// The edge is not occluded from the selected projection.
    Visible,
    /// The edge is behind another mesh triangle.
    Hidden,
}

/// A model-space edge classified for a drawing projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassifiedEdge {
    /// Projected edge start in model-space meters.
    pub start_m: [f64; 2],
    /// Projected edge end in model-space meters.
    pub end_m: [f64; 2],
    /// Visibility determined by the mesh depth test.
    pub visibility: LineVisibility,
}

#[derive(Debug, Clone, Copy)]
struct ViewPoint {
    uv: [f64; 2],
    depth_m: f64,
}

/// Project mesh edges and classify them against projected triangle boundaries.
///
/// Every non-adjacent projected triangle contributes the parameters at which its
/// three projected edges cross the projected mesh edge. The resulting sorted
/// parameter intervals are classified at their midpoints using interpolated edge
/// depth. Tessellation diagonals belonging to the same B-Rep face are omitted.
/// All tolerances are explicit so that nearly coincident projected boundaries do
/// not make output ordering dependent on input triangle order.
pub fn classify_hidden_lines(mesh: &MeshSet, projection: ProjectionKind) -> Vec<ClassifiedEdge> {
    let triangles: Vec<[ViewPoint; 3]> = mesh
        .indices
        .chunks_exact(3)
        .filter_map(|indices| {
            Some([
                view_point(*mesh.positions.get(indices[0] as usize)?, projection),
                view_point(*mesh.positions.get(indices[1] as usize)?, projection),
                view_point(*mesh.positions.get(indices[2] as usize)?, projection),
            ])
        })
        .collect();

    let mut edges: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (triangle_index, triangle) in mesh.indices.chunks_exact(3).enumerate() {
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            edges
                .entry(if a <= b { (a, b) } else { (b, a) })
                .or_default()
                .push(triangle_index);
        }
    }

    edges
        .into_iter()
        .filter_map(|((a, b), adjacent)| {
            if is_tessellation_diagonal(mesh, &adjacent) {
                return None;
            }
            let start = view_point(*mesh.positions.get(a as usize)?, projection);
            let end = view_point(*mesh.positions.get(b as usize)?, projection);
            if squared_distance(start.uv, end.uv)
                <= HIDDEN_LINE_PROJECTION_TOLERANCE_M * HIDDEN_LINE_PROJECTION_TOLERANCE_M
            {
                return None;
            }
            let mut split_parameters = vec![0.0, 1.0];
            for (triangle_index, triangle) in triangles.iter().enumerate() {
                if adjacent.contains(&triangle_index) {
                    continue;
                }
                collect_triangle_intersections(&mut split_parameters, start.uv, end.uv, triangle);
            }
            sort_and_deduplicate_parameters(&mut split_parameters);
            let boundary_parameters = split_parameters.clone();
            for interval in boundary_parameters.windows(2) {
                let [parameter_start, parameter_end] = *interval else {
                    continue;
                };
                if parameter_end - parameter_start <= HIDDEN_LINE_PARAMETER_TOLERANCE {
                    continue;
                }
                let midpoint_parameter = (parameter_start + parameter_end) * 0.5;
                let midpoint = interpolate_view_point(start, end, midpoint_parameter);
                for (triangle_index, triangle) in triangles.iter().enumerate() {
                    if adjacent.contains(&triangle_index)
                        || triangle_depth_at(triangle, midpoint.uv).is_none()
                    {
                        continue;
                    }
                    collect_depth_crossing(
                        &mut split_parameters,
                        start,
                        end,
                        triangle,
                        parameter_start,
                        parameter_end,
                    );
                }
            }
            sort_and_deduplicate_parameters(&mut split_parameters);

            let mut classified = Vec::new();
            for interval in split_parameters.windows(2) {
                let [parameter_start, parameter_end] = *interval else {
                    continue;
                };
                if parameter_end - parameter_start <= HIDDEN_LINE_PARAMETER_TOLERANCE {
                    continue;
                }
                let midpoint_parameter = (parameter_start + parameter_end) * 0.5;
                let midpoint = interpolate_view_point(start, end, midpoint_parameter);
                let hidden = triangles.iter().enumerate().any(|(index, triangle)| {
                    !adjacent.contains(&index)
                        && triangle_depth_at(triangle, midpoint.uv).is_some_and(|depth| {
                            depth > midpoint.depth_m + HIDDEN_LINE_DEPTH_TOLERANCE_M
                        })
                });
                let segment_start = interpolate_view_point(start, end, parameter_start);
                let segment_end = interpolate_view_point(start, end, parameter_end);
                append_classified_segment(
                    &mut classified,
                    segment_start.uv,
                    segment_end.uv,
                    if hidden {
                        LineVisibility::Hidden
                    } else {
                        LineVisibility::Visible
                    },
                );
            }
            // `filter_map` emits one value per source edge. Flattening here keeps
            // the edge ordering from the BTreeMap while preserving parameter order
            // within each edge.
            (!classified.is_empty()).then_some(classified)
        })
        .flatten()
        .collect()
}

fn collect_triangle_intersections(
    parameters: &mut Vec<f64>,
    edge_start: [f64; 2],
    edge_end: [f64; 2],
    triangle: &[ViewPoint; 3],
) {
    if barycentric_weights(triangle, triangle[0].uv).is_none() {
        return;
    }
    for (triangle_start, triangle_end) in [
        (triangle[0].uv, triangle[1].uv),
        (triangle[1].uv, triangle[2].uv),
        (triangle[2].uv, triangle[0].uv),
    ] {
        if let Some(parameter) =
            segment_intersection_parameter(edge_start, edge_end, triangle_start, triangle_end)
        {
            parameters.push(parameter);
        }
    }
}

fn collect_depth_crossing(
    parameters: &mut Vec<f64>,
    edge_start: ViewPoint,
    edge_end: ViewPoint,
    triangle: &[ViewPoint; 3],
    parameter_start: f64,
    parameter_end: f64,
) {
    let projected_start = interpolate_view_point(edge_start, edge_end, parameter_start);
    let projected_end = interpolate_view_point(edge_start, edge_end, parameter_end);
    let Some(triangle_depth_start) = triangle_depth_linear_at(triangle, projected_start.uv) else {
        return;
    };
    let Some(triangle_depth_end) = triangle_depth_linear_at(triangle, projected_end.uv) else {
        return;
    };
    let signed_start =
        triangle_depth_start - projected_start.depth_m - HIDDEN_LINE_DEPTH_TOLERANCE_M;
    let signed_end = triangle_depth_end - projected_end.depth_m - HIDDEN_LINE_DEPTH_TOLERANCE_M;
    let opposite_signs =
        (signed_start > 0.0 && signed_end < 0.0) || (signed_start < 0.0 && signed_end > 0.0);
    if !opposite_signs {
        return;
    }
    let denominator = signed_end - signed_start;
    if denominator.abs() <= HIDDEN_LINE_DEPTH_TOLERANCE_M {
        return;
    }
    let local_parameter = -signed_start / denominator;
    if local_parameter <= HIDDEN_LINE_PARAMETER_TOLERANCE
        || local_parameter >= 1.0 - HIDDEN_LINE_PARAMETER_TOLERANCE
    {
        return;
    }
    parameters.push(parameter_start + (parameter_end - parameter_start) * local_parameter);
}

fn segment_intersection_parameter(
    edge_start: [f64; 2],
    edge_end: [f64; 2],
    boundary_start: [f64; 2],
    boundary_end: [f64; 2],
) -> Option<f64> {
    let edge = subtract(edge_end, edge_start);
    let boundary = subtract(boundary_end, boundary_start);
    let offset = subtract(boundary_start, edge_start);
    let denominator = cross(edge, boundary);
    let denominator_tolerance =
        HIDDEN_LINE_PROJECTION_TOLERANCE_M * HIDDEN_LINE_PROJECTION_TOLERANCE_M;

    if denominator.abs() <= denominator_tolerance {
        // Parallel segments only meet in the collinear case. Their overlap is
        // bounded by projected triangle vertices, so add both edge parameters
        // that lie on the projected edge.
        if cross(offset, edge).abs() > denominator_tolerance {
            return None;
        }
        let edge_length_squared = dot(edge, edge);
        if edge_length_squared <= denominator_tolerance {
            return None;
        }
        let mut overlap_start =
            dot(subtract(boundary_start, edge_start), edge) / edge_length_squared;
        let mut overlap_end = dot(subtract(boundary_end, edge_start), edge) / edge_length_squared;
        if overlap_start > overlap_end {
            std::mem::swap(&mut overlap_start, &mut overlap_end);
        }
        let overlap_start = overlap_start.max(0.0);
        let overlap_end = overlap_end.min(1.0);
        if overlap_start <= overlap_end + HIDDEN_LINE_PARAMETER_TOLERANCE {
            // A single parameter is enough for a point overlap. For a proper
            // collinear overlap, the second endpoint is collected by the next
            // call in the same deterministic loop through triangle edges only
            // when it is a distinct triangle vertex; returning the first here
            // still creates a valid interval boundary.
            return Some(overlap_start);
        }
        return None;
    }

    let parameter = cross(offset, boundary) / denominator;
    let boundary_parameter = cross(offset, edge) / denominator;
    let accepted = -HIDDEN_LINE_PARAMETER_TOLERANCE..=1.0 + HIDDEN_LINE_PARAMETER_TOLERANCE;
    (accepted.contains(&parameter) && accepted.contains(&boundary_parameter))
        .then_some(parameter.clamp(0.0, 1.0))
}

fn sort_and_deduplicate_parameters(parameters: &mut Vec<f64>) {
    parameters.retain(|parameter| parameter.is_finite());
    parameters.sort_by(|a, b| a.total_cmp(b));
    let mut deduplicated = Vec::with_capacity(parameters.len());
    for parameter in parameters.iter().copied() {
        let parameter = parameter.clamp(0.0, 1.0);
        if deduplicated.last().map_or(true, |previous: &f64| {
            parameter - *previous > HIDDEN_LINE_PARAMETER_TOLERANCE
        }) {
            deduplicated.push(parameter);
        }
    }
    *parameters = deduplicated;
}

fn append_classified_segment(
    segments: &mut Vec<ClassifiedEdge>,
    start_m: [f64; 2],
    end_m: [f64; 2],
    visibility: LineVisibility,
) {
    if let Some(previous) = segments.last_mut() {
        if previous.visibility == visibility
            && squared_distance(previous.end_m, start_m)
                <= HIDDEN_LINE_PROJECTION_TOLERANCE_M * HIDDEN_LINE_PROJECTION_TOLERANCE_M
        {
            previous.end_m = end_m;
            return;
        }
    }
    segments.push(ClassifiedEdge {
        start_m,
        end_m,
        visibility,
    });
}

fn is_tessellation_diagonal(mesh: &MeshSet, adjacent: &[usize]) -> bool {
    adjacent.len() == 2
        && mesh.has_triangle_face_ids()
        && mesh.triangle_face_ids[adjacent[0]] == mesh.triangle_face_ids[adjacent[1]]
}

fn view_point(point: [f32; 3], projection: ProjectionKind) -> ViewPoint {
    let point = [point[0] as f64, point[1] as f64, point[2] as f64];
    let depth_m = match projection {
        ProjectionKind::Front => point[2],
        ProjectionKind::Top => point[1],
        ProjectionKind::Right => point[0],
        ProjectionKind::Isometric => 0.5 * point[0] - 0.35 * point[1] + point[2],
    };
    ViewPoint {
        uv: projection.project_point(point),
        depth_m,
    }
}

fn interpolate_view_point(start: ViewPoint, end: ViewPoint, parameter: f64) -> ViewPoint {
    ViewPoint {
        uv: [
            start.uv[0] + (end.uv[0] - start.uv[0]) * parameter,
            start.uv[1] + (end.uv[1] - start.uv[1]) * parameter,
        ],
        depth_m: start.depth_m + (end.depth_m - start.depth_m) * parameter,
    }
}

fn triangle_depth_at(triangle: &[ViewPoint; 3], point: [f64; 2]) -> Option<f64> {
    let weights = barycentric_weights(triangle, point)?;
    if weights
        .iter()
        .any(|weight| *weight < -HIDDEN_LINE_BARYCENTRIC_TOLERANCE)
    {
        return None;
    }

    Some(triangle_depth_from_weights(triangle, weights))
}

fn triangle_depth_linear_at(triangle: &[ViewPoint; 3], point: [f64; 2]) -> Option<f64> {
    let weights = barycentric_weights(triangle, point)?;
    Some(triangle_depth_from_weights(triangle, weights))
}

fn triangle_depth_from_weights(triangle: &[ViewPoint; 3], weights: [f64; 3]) -> f64 {
    weights[0] * triangle[0].depth_m
        + weights[1] * triangle[1].depth_m
        + weights[2] * triangle[2].depth_m
}

fn barycentric_weights(triangle: &[ViewPoint; 3], point: [f64; 2]) -> Option<[f64; 3]> {
    let [a, b, c] = *triangle;
    let denominator =
        (b.uv[1] - c.uv[1]) * (a.uv[0] - c.uv[0]) + (c.uv[0] - b.uv[0]) * (a.uv[1] - c.uv[1]);
    let denominator_tolerance =
        HIDDEN_LINE_PROJECTION_TOLERANCE_M * HIDDEN_LINE_PROJECTION_TOLERANCE_M;
    if !denominator.is_finite() || denominator.abs() <= denominator_tolerance {
        return None;
    }
    let wa = ((b.uv[1] - c.uv[1]) * (point[0] - c.uv[0])
        + (c.uv[0] - b.uv[0]) * (point[1] - c.uv[1]))
        / denominator;
    let wb = ((c.uv[1] - a.uv[1]) * (point[0] - c.uv[0])
        + (a.uv[0] - c.uv[0]) * (point[1] - c.uv[1]))
        / denominator;
    let wc = 1.0 - wa - wb;
    let weights = [wa, wb, wc];
    weights
        .iter()
        .all(|weight| weight.is_finite())
        .then_some(weights)
}

fn squared_distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}

fn subtract(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partially_occluded_mesh() -> MeshSet {
        MeshSet {
            positions: vec![
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [-0.5, -0.5, 1.0],
                [0.5, -0.5, 1.0],
                [0.0, 0.5, 1.0],
            ],
            normals: Vec::new(),
            indices: vec![0, 1, 2, 3, 4, 5],
            triangle_face_ids: vec![1, 2],
        }
    }

    fn depth_crossing_mesh() -> MeshSet {
        MeshSet {
            positions: vec![
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, -2.0, 0.0],
                [-2.0, -1.0, -1.0],
                [2.0, -1.0, 1.0],
                [0.0, 2.0, 0.0],
            ],
            normals: Vec::new(),
            indices: vec![0, 1, 2, 3, 4, 5],
            triangle_face_ids: vec![1, 2],
        }
    }

    fn horizontal_segments(lines: &[ClassifiedEdge]) -> Vec<ClassifiedEdge> {
        lines
            .iter()
            .copied()
            .filter(|line| {
                line.start_m[1].abs() <= HIDDEN_LINE_PROJECTION_TOLERANCE_M
                    && line.end_m[1].abs() <= HIDDEN_LINE_PROJECTION_TOLERANCE_M
            })
            .collect()
    }

    #[test]
    fn front_triangle_hides_edge_behind_it() {
        let mesh = MeshSet {
            positions: vec![
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [0.0, 1.0, 1.0],
                [-0.25, 0.0, 0.0],
                [0.25, 0.0, 0.0],
                [0.0, -0.25, 0.0],
            ],
            normals: Vec::new(),
            indices: vec![0, 1, 2, 3, 4, 5],
            triangle_face_ids: vec![1, 2],
        };
        let lines = classify_hidden_lines(&mesh, ProjectionKind::Front);
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.visibility == LineVisibility::Hidden)
                .count(),
            3
        );
    }

    #[test]
    fn partially_occluded_edge_is_visible_hidden_visible() {
        let lines = classify_hidden_lines(&partially_occluded_mesh(), ProjectionKind::Front);
        let horizontal = horizontal_segments(&lines);
        assert_eq!(horizontal.len(), 3);

        assert_eq!(horizontal[0].visibility, LineVisibility::Visible);
        assert_eq!(horizontal[1].visibility, LineVisibility::Hidden);
        assert_eq!(horizontal[2].visibility, LineVisibility::Visible);

        for (line, expected_start, expected_end) in [
            (&horizontal[0], -1.0, -0.25),
            (&horizontal[1], -0.25, 0.25),
            (&horizontal[2], 0.25, 1.0),
        ] {
            assert!((line.start_m[0] - expected_start).abs() <= 1.0e-9);
            assert!((line.end_m[0] - expected_end).abs() <= 1.0e-9);
            assert!(line.start_m[1].abs() <= HIDDEN_LINE_PROJECTION_TOLERANCE_M);
            assert!(line.end_m[1].abs() <= HIDDEN_LINE_PROJECTION_TOLERANCE_M);
        }
    }

    #[test]
    fn triangle_input_order_does_not_change_segments() {
        let mesh = partially_occluded_mesh();
        let reordered = MeshSet {
            positions: mesh.positions.clone(),
            normals: mesh.normals.clone(),
            indices: vec![3, 4, 5, 0, 1, 2],
            triangle_face_ids: vec![2, 1],
        };

        assert_eq!(
            classify_hidden_lines(&mesh, ProjectionKind::Front),
            classify_hidden_lines(&reordered, ProjectionKind::Front)
        );
    }

    #[test]
    fn depth_crossing_inside_one_projected_triangle_splits_edge() {
        let lines = classify_hidden_lines(&depth_crossing_mesh(), ProjectionKind::Front);
        let horizontal = horizontal_segments(&lines);
        assert_eq!(horizontal.len(), 2);
        assert_eq!(horizontal[0].visibility, LineVisibility::Visible);
        assert_eq!(horizontal[1].visibility, LineVisibility::Hidden);
        assert!((horizontal[0].start_m[0] + 1.0).abs() <= 1.0e-9);
        assert!(horizontal[0].end_m[0].abs() <= 1.0e-6);
        assert!(horizontal[1].start_m[0].abs() <= 1.0e-6);
        assert!((horizontal[1].end_m[0] - 1.0).abs() <= 1.0e-9);
    }

    #[test]
    fn same_face_tessellation_diagonal_is_omitted() {
        let mesh = MeshSet {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            normals: Vec::new(),
            indices: vec![0, 1, 2, 0, 2, 3],
            triangle_face_ids: vec![7, 7],
        };
        assert_eq!(classify_hidden_lines(&mesh, ProjectionKind::Front).len(), 4);
    }
}
