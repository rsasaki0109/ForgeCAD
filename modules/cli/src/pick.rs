//! `opencad pick` — headless viewport selection query.

use opencad_core::Result;
use opencad_feature::FeatureNode;
use opencad_geometry::{FaceDerivation, TopoRef};
use opencad_render::{OffscreenRenderer, PickResult, triangle_world_positions};
use serde::{Deserialize, Serialize};

use crate::mesh::{load_view_data, PREVIEW_HEIGHT, PREVIEW_WIDTH};
use crate::scene_query::{infer_face_refs, topo_ref_for_group};

/// Options for `opencad pick`.
#[derive(Debug, Clone, PartialEq)]
pub struct PickOptions {
    pub x: f64,
    pub y: f64,
    pub width: u32,
    pub height: u32,
}

impl Default for PickOptions {
    fn default() -> Self {
        Self {
            x: PREVIEW_WIDTH as f64 * 0.5,
            y: PREVIEW_HEIGHT as f64 * 0.5,
            width: PREVIEW_WIDTH,
            height: PREVIEW_HEIGHT,
        }
    }
}

/// Serializable pick target returned to agents and CLI `--json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PickTarget {
    None,
    SketchLine {
        line_index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        sketch_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        entity_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        entity_kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        segment_index: Option<usize>,
        construction: bool,
        start_m: [f32; 3],
        end_m: [f32; 3],
    },
    SolidTriangle {
        triangle_index: usize,
        vertices_m: [[f32; 3]; 3],
        #[serde(skip_serializing_if = "Option::is_none")]
        face_group_index: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        face_role: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        face_normal_m: Option<[f32; 3]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        face_centroid_m: Option<[f32; 3]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        kernel_face_id: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        inferred_feature_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        inferred_topo_ref_id: Option<String>,
    },
}

/// Summary returned by `opencad pick` and `opencad.pick_document`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PickSummary {
    pub x: f64,
    pub y: f64,
    pub width: u32,
    pub height: u32,
    pub overlay_line_count: usize,
    pub triangle_count: usize,
    pub selection: PickTarget,
}

pub fn pick_document(input: &str, options: &PickOptions) -> Result<PickSummary> {
    let data = load_view_data(input)?;
    let overlay = if data.overlay.is_empty() {
        None
    } else {
        Some(&data.overlay)
    };
    let renderer = OffscreenRenderer::new()?;
    let pick = renderer.pick_scene_at(
        &data.scene,
        overlay,
        options.x,
        options.y,
        options.width,
        options.height,
    )?;
    Ok(build_pick_summary(
        &data.scene,
        &data.overlay,
        pick,
        options,
        Some(&data.feature_nodes),
        &data.semantic_refs,
        &data.face_history,
    ))
}

pub fn build_pick_summary(
    scene: &opencad_render::RenderScene,
    overlay: &opencad_render::SketchOverlay,
    pick: PickResult,
    options: &PickOptions,
    feature_nodes: Option<&[FeatureNode]>,
    semantic_refs: &[TopoRef],
    face_history: &[FaceDerivation],
) -> PickSummary {
    let selection = match pick {
        PickResult::None => PickTarget::None,
        PickResult::SketchLine(line_index) => {
            let line = overlay.lines.get(line_index);
            let entity = overlay.pickable_line_at(line_index);
            PickTarget::SketchLine {
                line_index,
                sketch_id: entity.as_ref().map(|entity| entity.sketch_id.clone()),
                entity_id: entity.as_ref().map(|entity| entity.entity_id.clone()),
                entity_kind: entity.as_ref().map(|entity| entity.entity_kind.to_string()),
                segment_index: entity.and_then(|entity| entity.segment_index),
                construction: line.is_some_and(|line| line.construction),
                start_m: line.map(|line| line.start).unwrap_or([0.0; 3]),
                end_m: line.map(|line| line.end).unwrap_or([0.0; 3]),
            }
        }
        PickResult::SolidTriangle(triangle_index) => {
            let face = scene.face_group_at(triangle_index);
            let inferred = face.and_then(|face| {
                feature_nodes.map(|nodes| infer_face_refs(nodes, face))
            });
            PickTarget::SolidTriangle {
                triangle_index,
                vertices_m: triangle_world_positions(scene, triangle_index)
                    .unwrap_or([[0.0; 3]; 3]),
                face_group_index: face.map(|face| face.index),
                face_role: face.map(|face| face.role.as_str().to_string()),
                face_normal_m: face.map(|face| face.normal),
                face_centroid_m: face.map(|face| face.centroid),
                kernel_face_id: face.and_then(|face| face.kernel_face_id),
                inferred_feature_id: inferred.as_ref().and_then(|(id, _)| id.clone()),
                inferred_topo_ref_id: face.and_then(|face| {
                    inferred.as_ref().and_then(|refs| {
                        topo_ref_for_group(face, refs, semantic_refs, face_history)
                    })
                }),
            }
        }
    };

    PickSummary {
        x: options.x,
        y: options.y,
        width: options.width,
        height: options.height,
        overlay_line_count: overlay.lines.len(),
        triangle_count: scene.triangle_count(),
        selection,
    }
}

pub fn print_summary(summary: &PickSummary) {
    println!("pick_x: {}", summary.x);
    println!("pick_y: {}", summary.y);
    println!("viewport: {}x{}", summary.width, summary.height);
    println!("overlay_lines: {}", summary.overlay_line_count);
    println!("triangles: {}", summary.triangle_count);
    match &summary.selection {
        PickTarget::None => println!("selection: none"),
        PickTarget::SketchLine {
            line_index,
            sketch_id,
            entity_id,
            construction,
            ..
        } => {
            println!("selection: sketch_line");
            println!("line_index: {line_index}");
            if let Some(sketch_id) = sketch_id {
                println!("sketch_id: {sketch_id}");
            }
            if let Some(entity_id) = entity_id {
                println!("entity_id: {entity_id}");
            }
            println!("construction: {construction}");
        }
        PickTarget::SolidTriangle {
            triangle_index,
            face_role,
            kernel_face_id,
            inferred_feature_id,
            inferred_topo_ref_id,
            ..
        } => {
            println!("selection: solid_triangle");
            println!("triangle_index: {triangle_index}");
            if let Some(role) = face_role {
                println!("face_role: {role}");
            }
            if let Some(kernel_face_id) = kernel_face_id {
                println!("kernel_face_id: {kernel_face_id}");
            }
            if let Some(feature_id) = inferred_feature_id {
                println!("inferred_feature_id: {feature_id}");
            }
            if let Some(topo_ref_id) = inferred_topo_ref_id {
                println!("inferred_topo_ref_id: {topo_ref_id}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::write_bracket_fixture_at;
    use tempfile::tempdir;

    #[test]
    fn pick_center_hits_solid_triangle() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("bracket.ocad.d");
        write_bracket_fixture_at(&path);

        let summary = pick_document(path.to_str().expect("path"), &PickOptions::default())
            .expect("pick");
        assert!(summary.triangle_count > 0);
        assert!(summary.overlay_line_count > 0);
        assert!(matches!(
            summary.selection,
            PickTarget::SolidTriangle {
                face_role: Some(_),
                inferred_feature_id: Some(_),
                ..
            } | PickTarget::SketchLine { .. }
        ));
    }

    #[test]
    fn pick_summary_maps_line_index_to_entity_id() {
        use opencad_feature::{apply_parameters, bracket_base_plate};
        use opencad_graph::{bracket_parameters, evaluate_param_graph};
        use opencad_render::{build_sketch_overlay, PickResult, RenderScene};
        use opencad_sketch::Sketch;

        let mut model = bracket_base_plate().expect("model");
        let params = bracket_parameters();
        apply_parameters(&mut model, &params).expect("apply");
        let values = evaluate_param_graph(&params).expect("eval");
        let sketches: Vec<Sketch> = model.sketches.values().cloned().collect();
        let overlay = build_sketch_overlay(&sketches, &values).expect("overlay");
        let line_index = overlay
            .lines
            .iter()
            .position(|line| line.entity_id.as_deref() == Some("ent:e0"))
            .expect("ent:e0 overlay line");
        let scene = RenderScene::from_mesh_set(&opencad_geometry::MeshSet::box_prism(0.08, 0.006))
            .expect("scene");
        let summary = build_pick_summary(
            &scene,
            &overlay,
            PickResult::SketchLine(line_index),
            &PickOptions::default(),
            None,
            &[],
            &[],
        );
        let PickTarget::SketchLine {
            sketch_id,
            entity_id,
            entity_kind,
            segment_index,
            ..
        } = summary.selection
        else {
            panic!("expected sketch line");
        };
        assert_eq!(sketch_id.as_deref(), Some("sketch:base"));
        assert_eq!(entity_id.as_deref(), Some("ent:e0"));
        assert_eq!(entity_kind.as_deref(), Some("line"));
        assert!(segment_index.is_none());
    }

    #[test]
    fn pick_corner_returns_none() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("bracket.ocad.d");
        write_bracket_fixture_at(&path);

        let summary = pick_document(
            path.to_str().expect("path"),
            &PickOptions {
                x: 0.0,
                y: 0.0,
                ..PickOptions::default()
            },
        )
        .expect("pick");
        assert!(matches!(summary.selection, PickTarget::None));
    }
}
