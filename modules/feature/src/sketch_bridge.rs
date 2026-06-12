//! Convert solved sketch profiles into kernel-neutral wire input.

use indexmap::IndexMap;

use opencad_core::{EntityId, OpenCadError, Result};
use opencad_geometry::SolvedSketch;
use opencad_sketch::{
    entity::{expand_rectangle, Coord, LineEntity, SketchEntity},
    Profile, ProfileKind, Sketch,
};

const CIRCLE_SEGMENTS: usize = 32;

/// Expand rectangle helpers, then refresh profile detection.
pub fn prepare_sketch(sketch: &mut Sketch) -> Result<()> {
    let rectangles: Vec<_> = sketch
        .entities
        .iter()
        .filter_map(|e| match e {
            SketchEntity::Rectangle(r) if !r.corner_ids.is_empty() => Some(r.clone()),
            _ => None,
        })
        .collect();

    if !rectangles.is_empty() {
        let mut expanded = Vec::new();
        for rect in rectangles {
            expanded.extend(expand_rectangle(&rect)?);
        }
        sketch.entities.retain(|e| !matches!(e, SketchEntity::Rectangle(_)));
        for entity in expanded {
            sketch.add_entity(entity)?;
        }
    }

    sketch.update_profiles()?;
    Ok(())
}

/// Build a `SolvedSketch` from a closed profile reference.
pub fn profile_to_solved(sketch: &Sketch, profile_ref: &str) -> Result<SolvedSketch> {
    let profile = find_profile(sketch, profile_ref)?;
    if profile.kind != ProfileKind::Closed {
        return Err(OpenCadError::validation(format!(
            "profile '{profile_ref}' is not closed"
        )));
    }

    let points = if profile.entity_ids.len() == 1 {
        circle_profile_points(sketch, &profile.entity_ids[0])?
    } else {
        line_loop_points(sketch, profile)?
    };

    if points.len() < 3 {
        return Err(OpenCadError::validation(format!(
            "profile '{profile_ref}' needs at least three points"
        )));
    }

    Ok(SolvedSketch {
        profile_ref: profile_ref.into(),
        points,
        closed: true,
    })
}

fn find_profile<'a>(sketch: &'a Sketch, profile_ref: &str) -> Result<&'a Profile> {
    sketch
        .profiles
        .iter()
        .find(|p| {
            p.profile_ref.as_deref() == Some(profile_ref)
                || p.id == profile_ref
                || format!("{}/profile:outer", sketch.id) == profile_ref
        })
        .ok_or_else(|| OpenCadError::not_found(format!("profile '{profile_ref}'")))
}

fn point_coord(sketch: &Sketch, point_id: &EntityId) -> Result<[f64; 2]> {
    let entity = sketch
        .find_entity(point_id.as_str())
        .ok_or_else(|| OpenCadError::not_found(format!("point '{}'", point_id.as_str())))?;
    let SketchEntity::Point(point) = entity else {
        return Err(OpenCadError::validation(format!(
            "entity '{}' is not a point",
            point_id.as_str()
        )));
    };
    match (&point.x, &point.y) {
        (Coord::Literal(x), Coord::Literal(y)) => Ok([*x, *y]),
        _ => Err(OpenCadError::validation(format!(
            "point '{}' must have literal coordinates; solve the sketch first",
            point_id.as_str()
        ))),
    }
}

fn line_loop_points(sketch: &Sketch, profile: &Profile) -> Result<Vec<[f64; 2]>> {
    let lines: IndexMap<String, &LineEntity> = sketch
        .entities
        .iter()
        .filter_map(|e| match e {
            SketchEntity::Line(l) => Some((l.base.id.as_str().to_string(), l)),
            _ => None,
        })
        .collect();

    let profile_lines: Vec<&LineEntity> = profile
        .entity_ids
        .iter()
        .map(|id| {
            lines
                .get(id.as_str())
                .copied()
                .ok_or_else(|| OpenCadError::not_found(format!("line '{}'", id.as_str())))
        })
        .collect::<Result<Vec<_>>>()?;

    if profile_lines.is_empty() {
        return Err(OpenCadError::validation("profile has no line entities"));
    }

    let first_line = profile_lines[0];
    let start = first_line.start.clone();
    let mut ordered = vec![point_coord(sketch, &first_line.start)?];
    let mut current_end = first_line.end.clone();
    ordered.push(point_coord(sketch, &current_end)?);
    let mut remaining: Vec<&LineEntity> = profile_lines[1..].to_vec();

    while current_end != start {
        let Some(idx) = remaining.iter().position(|line| {
            line.start == current_end || line.end == current_end
        }) else {
            break;
        };
        let line = remaining.remove(idx);
        if line.start == current_end {
            current_end = line.end.clone();
        } else {
            current_end = line.start.clone();
        }
        ordered.push(point_coord(sketch, &current_end)?);
        if ordered.len() > profile_lines.len() + 1 {
            return Err(OpenCadError::validation(
                "profile loop traversal did not close cleanly",
            ));
        }
    }

    if current_end != start {
        return Err(OpenCadError::validation(
            "profile loop does not close on the first point",
        ));
    }

    if ordered.len() > 1 && ordered.last() == ordered.first() {
        ordered.pop();
    }

    Ok(ordered)
}

fn circle_profile_points(sketch: &Sketch, circle_id: &EntityId) -> Result<Vec<[f64; 2]>> {
    let entity = sketch
        .find_entity(circle_id.as_str())
        .ok_or_else(|| OpenCadError::not_found(format!("circle '{}'", circle_id.as_str())))?;
    let SketchEntity::Circle(circle) = entity else {
        return Err(OpenCadError::validation(format!(
            "entity '{}' is not a circle",
            circle_id.as_str()
        )));
    };
    let center = point_coord(sketch, &circle.center)?;
    let radius = match &circle.radius {
        Coord::Literal(r) => *r,
        _ => {
            return Err(OpenCadError::validation(
                "circle radius must be a literal after solving",
            ))
        }
    };

    let mut points = Vec::with_capacity(CIRCLE_SEGMENTS);
    for i in 0..CIRCLE_SEGMENTS {
        let angle = std::f64::consts::TAU * i as f64 / CIRCLE_SEGMENTS as f64;
        points.push([
            center[0] + radius * angle.cos(),
            center[1] + radius * angle.sin(),
        ]);
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencad_sketch::{
        constraint::{Constraint, DistanceTarget},
        entity::{EntityBase, LineEntity, PointEntity},
        workplane::Workplane,
        Sketch,
    };
    use opencad_core::{ConstraintId, EntityId, Expression, SketchId};

    fn solved_rectangle_sketch() -> Sketch {
        let mut sketch = Sketch::new(
            SketchId::new("sketch:base").expect("id"),
            "Base",
            Workplane::xy(),
        );

        let corners = ["ent:c0", "ent:c1", "ent:c2", "ent:c3"];
        let edges = ["ent:e0", "ent:e1", "ent:e2", "ent:e3"];
        for (id, x, y) in [
            (corners[0], 0.0, 0.0),
            (corners[1], 0.08, 0.0),
            (corners[2], 0.08, 0.06),
            (corners[3], 0.0, 0.06),
        ] {
            sketch
                .add_entity(SketchEntity::Point(PointEntity {
                    base: EntityBase {
                        id: EntityId::new(id).expect("id"),
                        construction: false,
                    },
                    x: Coord::literal(x),
                    y: Coord::literal(y),
                }))
                .expect("point");
        }
        for (id, start, end) in [
            (edges[0], corners[0], corners[1]),
            (edges[1], corners[1], corners[2]),
            (edges[2], corners[2], corners[3]),
            (edges[3], corners[3], corners[0]),
        ] {
            sketch
                .add_entity(SketchEntity::Line(LineEntity {
                    base: EntityBase {
                        id: EntityId::new(id).expect("id"),
                        construction: false,
                    },
                    start: EntityId::new(start).expect("id"),
                    end: EntityId::new(end).expect("id"),
                }))
                .expect("line");
        }
        sketch
            .add_constraint(Constraint::Distance {
                id: ConstraintId::new("con:w").expect("id"),
                target: DistanceTarget::LineLength {
                    line: EntityId::new(edges[0]).expect("id"),
                },
                expr: Expression::new("80 mm").expect("expr"),
            })
            .expect("constraint");
        sketch.update_profiles().expect("profiles");
        sketch
    }

    #[test]
    fn profile_to_solved_rectangle() {
        let sketch = solved_rectangle_sketch();
        let solved = profile_to_solved(&sketch, "sketch:base/profile:outer").expect("solved");
        assert_eq!(solved.points.len(), 4);
        assert!(solved.closed);
    }
}
