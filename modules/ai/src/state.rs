//! In-memory design state for agent operations (no file I/O).

use std::collections::BTreeMap;

use opencad_assembly::AssemblyModel;
use opencad_core::{sha256_hex, Result};
use opencad_drawing::DrawingModel;
use opencad_feature::FeatureNode;
use opencad_geometry::TopoRef;
use opencad_graph::{build_summary, diff_param_graphs, diff_semantic_refs, DesignDiff, ParamGraph};
use serde::Serialize;
use serde_json::{Map, Value};

/// Digest algorithm used for DesignPatch document revisions.
pub const DESIGN_STATE_REVISION_ALGORITHM: &str = "sha256";
/// Versioned canonical representation used by [`design_state_revision`].
pub const DESIGN_STATE_REVISION_VERSION: &str = "musubicad.design-state.v1";

/// Serializable design intent used by in-memory agent operations.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DesignState {
    pub parameters: ParamGraph,
    pub feature_nodes: Vec<FeatureNode>,
    pub semantic_refs: Vec<TopoRef>,
    pub assembly: Option<AssemblyModel>,
    pub drawing: Option<DrawingModel>,
}

/// Compute the deterministic revision for the complete state exposed to
/// `DesignPatch` operations.
///
/// The version is included in the hashed envelope so a future canonical
/// representation cannot silently compare equal to this one. Object keys are
/// normalized before serialization; vector order is retained because source
/// ordering can be meaningful to regeneration and document identity.
pub fn design_state_revision(state: &DesignState) -> Result<String> {
    let bytes = canonical_design_state_bytes(state)?;
    Ok(sha256_hex(&bytes))
}

/// Serialize the canonical, versioned DesignState revision payload.
pub fn canonical_design_state_bytes(state: &DesignState) -> Result<Vec<u8>> {
    let canonical_state = canonicalize_json(serde_json::to_value(state)?);
    let mut payload = Map::new();
    payload.insert("state".into(), canonical_state);
    payload.insert(
        "version".into(),
        Value::String(DESIGN_STATE_REVISION_VERSION.into()),
    );
    Ok(serde_json::to_vec(&canonicalize_json(Value::Object(
        payload,
    )))?)
}

/// Canonicalize a serializable value for semantic identity comparisons.
pub(crate) fn canonical_json_bytes<T: Serialize>(value: &T) -> Option<Vec<u8>> {
    let value = serde_json::to_value(value).ok()?;
    serde_json::to_vec(&canonicalize_json(value)).ok()
}

pub(crate) fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<_> = object.into_iter().collect();
            keys.sort_by(|left, right| left.0.cmp(&right.0));
            let mut canonical = Map::new();
            for (key, value) in keys {
                canonical.insert(key, canonicalize_json(value));
            }
            Value::Object(canonical)
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalize_json).collect()),
        scalar => scalar,
    }
}

impl DesignState {
    pub fn new(parameters: ParamGraph, feature_nodes: Vec<FeatureNode>) -> Self {
        Self {
            parameters,
            feature_nodes,
            semantic_refs: Vec::new(),
            assembly: None,
            drawing: None,
        }
    }

    pub fn with_semantic_refs(
        parameters: ParamGraph,
        feature_nodes: Vec<FeatureNode>,
        semantic_refs: Vec<TopoRef>,
    ) -> Self {
        Self {
            parameters,
            feature_nodes,
            semantic_refs,
            assembly: None,
            drawing: None,
        }
    }

    pub fn with_assembly(
        parameters: ParamGraph,
        feature_nodes: Vec<FeatureNode>,
        semantic_refs: Vec<TopoRef>,
        assembly: Option<AssemblyModel>,
    ) -> Self {
        Self {
            parameters,
            feature_nodes,
            semantic_refs,
            assembly,
            drawing: None,
        }
    }

    pub fn with_models(
        parameters: ParamGraph,
        feature_nodes: Vec<FeatureNode>,
        semantic_refs: Vec<TopoRef>,
        assembly: Option<AssemblyModel>,
        drawing: Option<DrawingModel>,
    ) -> Self {
        Self {
            parameters,
            feature_nodes,
            semantic_refs,
            assembly,
            drawing,
        }
    }
}

/// Compare two design states and return a semantic diff.
pub fn diff_design_state(before: &DesignState, after: &DesignState) -> DesignDiff {
    let mut changes = diff_param_graphs(&before.parameters, &after.parameters);
    changes.extend(diff_feature_nodes(
        &before.feature_nodes,
        &after.feature_nodes,
    ));
    changes.extend(diff_semantic_refs(
        &before.semantic_refs,
        &after.semantic_refs,
    ));
    if let (Some(before_assembly), Some(after_assembly)) = (&before.assembly, &after.assembly) {
        let assembly_diff = crate::assembly::diff_assembly_models(before_assembly, after_assembly);
        changes.extend(assembly_diff.changes);
    }
    if let (Some(before_drawing), Some(after_drawing)) = (&before.drawing, &after.drawing) {
        changes.extend(crate::drawing::diff_drawing_models(before_drawing, after_drawing).changes);
    }
    DesignDiff::semantic(build_summary(&changes), changes)
}

fn diff_feature_nodes(
    before: &[FeatureNode],
    after: &[FeatureNode],
) -> Vec<opencad_graph::SemanticChange> {
    use opencad_graph::SemanticChange;

    let before_map: BTreeMap<String, &FeatureNode> =
        before.iter().map(|node| (node.id.clone(), node)).collect();
    let after_map: BTreeMap<String, &FeatureNode> =
        after.iter().map(|node| (node.id.clone(), node)).collect();

    let mut ids = BTreeMap::new();
    for id in before_map.keys() {
        ids.insert(id.clone(), ());
    }
    for id in after_map.keys() {
        ids.insert(id.clone(), ());
    }

    let mut changes = Vec::new();
    for id in ids.keys() {
        match (before_map.get(id), after_map.get(id)) {
            (Some(_), None) => changes.push(SemanticChange::FeatureRemoved { id: id.clone() }),
            (None, Some(after_node)) => changes.push(SemanticChange::FeatureAdded {
                id: id.clone(),
                feature_type: after_node.definition.feature_type().to_string(),
            }),
            (Some(before_node), Some(after_node)) if before_node != after_node => {
                changes.extend(diff_feature_node(before_node, after_node));
            }
            _ => {}
        }
    }
    changes
}

fn diff_feature_node(
    before: &FeatureNode,
    after: &FeatureNode,
) -> Vec<opencad_graph::SemanticChange> {
    use opencad_graph::SemanticChange;

    let mut changes = Vec::new();
    if before.name != after.name {
        changes.push(SemanticChange::FeatureModified {
            id: before.id.clone(),
            field: "name".into(),
            before: before.name.clone(),
            after: after.name.clone(),
        });
    }
    if before.suppressed != after.suppressed {
        changes.push(SemanticChange::FeatureModified {
            id: before.id.clone(),
            field: "suppressed".into(),
            before: before.suppressed.to_string(),
            after: after.suppressed.to_string(),
        });
    }
    if before.definition != after.definition {
        changes.push(SemanticChange::FeatureModified {
            id: before.id.clone(),
            field: "definition".into(),
            before: serde_json::to_string(&before.definition).unwrap_or_default(),
            after: serde_json::to_string(&after.definition).unwrap_or_default(),
        });
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencad_core::{DocumentId, SheetId, ViewId};
    use opencad_drawing::{DrawingModel, DrawingView, ModelReference, ProjectionKind, Sheet};
    use opencad_feature::bracket_with_hole;
    use opencad_graph::ParameterEntry;

    fn graph_with_order(order: &[&str]) -> ParamGraph {
        let mut graph = ParamGraph::new();
        for id in order {
            graph
                .add_parameter(ParameterEntry::new(
                    *id,
                    id.strip_prefix("param:").unwrap_or(id),
                    if *id == "param:width" {
                        "80 mm"
                    } else {
                        "60 mm"
                    },
                ))
                .expect("parameter");
        }
        graph
            .add_dependency("param:width", "param:height")
            .expect("edge");
        graph
    }

    fn drawing_with_scale(scale: f64) -> DrawingModel {
        let mut sheet = Sheet::a4_portrait(SheetId::new("sheet:main").expect("sheet"), "Main");
        sheet.views.push(DrawingView::new(
            ViewId::new("view:front").expect("view"),
            "Front",
            ModelReference::new(
                "parts/bracket.ocad.d",
                DocumentId::new("doc:bracket").expect("document"),
            ),
            ProjectionKind::Front,
            scale,
            [0.05, 0.06],
        ));
        DrawingModel {
            sheets: vec![sheet],
        }
    }

    #[test]
    fn canonical_revision_ignores_insertion_order_of_design_collections() {
        let part = bracket_with_hole().expect("part");
        let first_nodes: Vec<_> = part.nodes.values().cloned().collect();
        let second_nodes = first_nodes.clone();

        let first = DesignState::with_models(
            graph_with_order(&["param:width", "param:height"]),
            first_nodes,
            Vec::new(),
            None,
            Some(drawing_with_scale(1.0)),
        );
        let second = DesignState::with_models(
            graph_with_order(&["param:height", "param:width"]),
            second_nodes,
            Vec::new(),
            None,
            Some(drawing_with_scale(1.0)),
        );

        assert_eq!(
            canonical_design_state_bytes(&first).expect("canonical bytes"),
            canonical_design_state_bytes(&second).expect("canonical bytes")
        );
        assert_eq!(
            design_state_revision(&first).expect("revision"),
            design_state_revision(&second).expect("revision")
        );
    }

    #[test]
    fn canonical_revision_preserves_feature_source_order() {
        let part = bracket_with_hole().expect("part");
        let first_nodes: Vec<_> = part.nodes.values().cloned().collect();
        let mut reordered_nodes = first_nodes.clone();
        reordered_nodes.reverse();
        let first = DesignState::new(ParamGraph::new(), first_nodes);
        let reordered = DesignState::new(ParamGraph::new(), reordered_nodes);

        assert_ne!(
            canonical_design_state_bytes(&first).expect("canonical bytes"),
            canonical_design_state_bytes(&reordered).expect("canonical bytes")
        );
        assert_ne!(
            design_state_revision(&first).expect("revision"),
            design_state_revision(&reordered).expect("revision")
        );
    }

    #[test]
    fn canonical_revision_preserves_source_geometry_identity() {
        let first = DesignState::with_models(
            ParamGraph::new(),
            Vec::new(),
            Vec::new(),
            None,
            Some(drawing_with_scale(1.0)),
        );
        let second = DesignState::with_models(
            ParamGraph::new(),
            Vec::new(),
            Vec::new(),
            None,
            Some(drawing_with_scale(1.0 + 1.0e-12)),
        );

        assert_ne!(
            design_state_revision(&first).expect("revision"),
            design_state_revision(&second).expect("revision")
        );
    }
}
