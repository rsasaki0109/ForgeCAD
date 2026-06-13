//! Tessellated scene queries for agents (`list_overlay_lines`, `list_face_groups`).

use opencad_ai::query::{FaceGroupInfo, OverlayLineInfo, SceneQueryContext};
use opencad_desktop::{infer_face_refs, topo_ref_for_group, ViewData};
use opencad_render::{FaceGroup, RenderScene, SketchOverlay};

pub fn build_scene_query_context(data: &ViewData) -> SceneQueryContext {
    SceneQueryContext {
        overlay_lines: list_overlay_line_infos(&data.overlay),
        face_groups: list_face_group_infos(
            &data.scene,
            &data.feature_nodes,
            &data.semantic_refs,
            &data.face_history,
        ),
    }
}

pub fn list_overlay_line_infos(overlay: &SketchOverlay) -> Vec<OverlayLineInfo> {
    overlay
        .lines
        .iter()
        .enumerate()
        .map(|(line_index, line)| {
            let entity = overlay.pickable_line_at(line_index);
            OverlayLineInfo {
                line_index,
                sketch_id: entity
                    .as_ref()
                    .map(|entity| entity.sketch_id.clone())
                    .or_else(|| line.sketch_id.clone()),
                entity_id: entity
                    .as_ref()
                    .map(|entity| entity.entity_id.clone())
                    .or_else(|| line.entity_id.clone()),
                entity_kind: entity.as_ref().map(|entity| entity.entity_kind.to_string()),
                segment_index: entity.and_then(|entity| entity.segment_index),
                construction: line.construction,
                start_m: line.start,
                end_m: line.end,
            }
        })
        .collect()
}

pub fn list_face_group_infos(
    scene: &RenderScene,
    feature_nodes: &[opencad_feature::FeatureNode],
    semantic_refs: &[opencad_geometry::TopoRef],
    face_history: &[opencad_geometry::FaceDerivation],
) -> Vec<FaceGroupInfo> {
    let mut items: Vec<FaceGroupInfo> = scene
        .face_catalog
        .groups
        .iter()
        .map(|group| face_group_info(group, feature_nodes, semantic_refs, face_history))
        .collect();
    items.sort_by_key(|item| item.face_group_index);
    items
}

fn face_group_info(
    group: &FaceGroup,
    feature_nodes: &[opencad_feature::FeatureNode],
    semantic_refs: &[opencad_geometry::TopoRef],
    face_history: &[opencad_geometry::FaceDerivation],
) -> FaceGroupInfo {
    let inferred = infer_face_refs(feature_nodes, group);
    let inferred_feature_id = inferred.0.clone();
    FaceGroupInfo {
        face_group_index: group.index,
        face_role: group.role.as_str().to_string(),
        triangle_count: group.triangle_count,
        face_normal_m: group.normal,
        face_centroid_m: group.centroid,
        kernel_face_id: group.kernel_face_id,
        inferred_feature_id,
        inferred_topo_ref_id: topo_ref_for_group(group, &inferred, semantic_refs, face_history),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::write_bracket_fixture_at;
    use opencad_feature::{apply_parameters, bracket_base_plate};
    use opencad_graph::{bracket_parameters, evaluate_param_graph};
    use opencad_render::build_sketch_overlay;
    use opencad_sketch::Sketch;
    use tempfile::tempdir;

    #[test]
    fn lists_overlay_lines_with_entity_mapping() {
        let mut model = bracket_base_plate().expect("model");
        let params = bracket_parameters();
        apply_parameters(&mut model, &params).expect("apply");
        let values = evaluate_param_graph(&params).expect("eval");
        let sketches: Vec<Sketch> = model.sketches.values().cloned().collect();
        let overlay = build_sketch_overlay(&sketches, &values).expect("overlay");

        let items = list_overlay_line_infos(&overlay);
        assert!(!items.is_empty());
        assert!(items
            .iter()
            .any(|item| item.entity_id.as_deref() == Some("ent:e0")));
        assert!(items
            .iter()
            .all(|item| item.line_index < overlay.lines.len()));
    }

    #[test]
    fn lists_face_groups_from_bracket_document() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("bracket.ocad.d");
        write_bracket_fixture_at(&path);
        let data = crate::mesh::load_view_data(path.to_str().expect("path")).expect("view");

        let items = list_face_group_infos(
            &data.scene,
            &data.feature_nodes,
            &data.semantic_refs,
            &data.face_history,
        );
        assert!(!items.is_empty());
        assert!(items.iter().any(|item| item.face_role == "top"));
        assert!(items.iter().any(|item| item.kernel_face_id.is_some()));
        assert!(items.iter().any(|item| item
            .inferred_topo_ref_id
            .as_deref()
            .map(|id| id.starts_with("ref:face:kernel_"))
            .unwrap_or(false)));
    }
}
