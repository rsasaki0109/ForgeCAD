//! Tessellated scene queries for agents (`list_overlay_lines`, `list_face_groups`).

use opencad_ai::query::{FaceGroupInfo, OverlayLineInfo, SceneQueryContext};
use opencad_feature::FeatureNode;
use opencad_geometry::{resolve_topo_ref_id_with_history, FaceDerivation, TopoRef};
use opencad_render::{FaceGroup, FaceRole, RenderScene, SketchOverlay};

use crate::mesh::ViewData;

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
    feature_nodes: &[FeatureNode],
    semantic_refs: &[TopoRef],
    face_history: &[FaceDerivation],
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
    feature_nodes: &[FeatureNode],
    semantic_refs: &[TopoRef],
    face_history: &[FaceDerivation],
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

pub(crate) fn topo_ref_for_group(
    group: &FaceGroup,
    inferred: &(Option<String>, Option<String>),
    semantic_refs: &[TopoRef],
    face_history: &[FaceDerivation],
) -> Option<String> {
    if let Some(kernel_face_id) = group.kernel_face_id {
        let direct = resolve_topo_ref_id_with_history(semantic_refs, kernel_face_id, face_history);
        if direct
            .as_deref()
            .is_some_and(|ref_id| !ref_id.starts_with("ref:face:kernel_"))
        {
            return direct;
        }

        if let Some(feature_id) = inferred.0.as_deref() {
            if let Some(custom) = semantic_refs.iter().find(|topo_ref| {
                topo_ref.semantic.role.as_deref() == Some(group.role.as_str())
                    && topo_ref.semantic.created_by == feature_id
                    && !topo_ref.ref_id.as_str().starts_with("ref:face:kernel_")
            }) {
                return Some(custom.ref_id.as_str().to_string());
            }
        }

        return direct;
    }
    inferred.1.clone()
}

pub(crate) fn infer_face_refs(
    features: &[FeatureNode],
    face: &FaceGroup,
) -> (Option<String>, Option<String>) {
    let feature_id = match face.role {
        FaceRole::Cylindrical => find_feature_by_type(features, "hole"),
        FaceRole::Top => find_feature_by_type(features, "fillet")
            .or_else(|| find_feature_by_type(features, "chamfer"))
            .or_else(|| find_feature_by_type(features, "extrude")),
        FaceRole::Bottom | FaceRole::PosX | FaceRole::NegX | FaceRole::PosY | FaceRole::NegY => {
            find_feature_by_type(features, "extrude")
        }
        FaceRole::Other => None,
    };
    let topo_ref_id = feature_id
        .as_deref()
        .and_then(|feature_id| infer_topo_ref_id(feature_id, face.role));
    (feature_id, topo_ref_id)
}

fn find_feature_by_type(features: &[FeatureNode], feature_type: &str) -> Option<String> {
    features
        .iter()
        .find(|node| node.definition.feature_type() == feature_type)
        .map(|node| node.id.clone())
}

fn infer_topo_ref_id(feature_id: &str, role: FaceRole) -> Option<String> {
    let suffix = match role {
        FaceRole::Top => "top",
        FaceRole::Bottom => "bottom",
        FaceRole::Cylindrical => "wall",
        FaceRole::PosX => "pos_x",
        FaceRole::NegX => "neg_x",
        FaceRole::PosY => "pos_y",
        FaceRole::NegY => "neg_y",
        FaceRole::Other => "other",
    };
    let stem = feature_id.strip_prefix("feature:").unwrap_or(feature_id);
    Some(format!("ref:face:{stem}_{suffix}"))
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
