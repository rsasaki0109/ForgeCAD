//! DesignPatch operations (Task-142+).

use opencad_core::{OpenCadError, Result};
use opencad_feature::{FeatureDefinition, FeatureNode};
use opencad_graph::ParamGraph;
use serde::{Deserialize, Serialize};

/// Supported feature expression fields for patch operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureExprField {
    LengthExpr,
    DepthExpr,
    RadiusExpr,
    DistanceExpr,
}

impl FeatureExprField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LengthExpr => "length_expr",
            Self::DepthExpr => "depth_expr",
            Self::RadiusExpr => "radius_expr",
            Self::DistanceExpr => "distance_expr",
        }
    }

    pub fn parse(field: &str) -> Result<Self> {
        match field {
            "length_expr" => Ok(Self::LengthExpr),
            "depth_expr" => Ok(Self::DepthExpr),
            "radius_expr" => Ok(Self::RadiusExpr),
            "distance_expr" => Ok(Self::DistanceExpr),
            _ => Err(OpenCadError::validation(format!(
                "unsupported feature field '{field}'; expected 'length_expr', 'depth_expr', 'radius_expr', or 'distance_expr'"
            ))),
        }
    }
}

/// A single patch operation against design intent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatchOperation {
    SetParameter { id: String, expr: String },
    SetFeatureExpr {
        feature_id: String,
        field: String,
        expr: String,
    },
}

/// Semantic patch applied by agents or CLI tooling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignPatch {
    pub operations: Vec<PatchOperation>,
}

impl DesignPatch {
    pub fn new(operations: Vec<PatchOperation>) -> Self {
        Self { operations }
    }

    pub fn set_parameter(id: impl Into<String>, expr: impl Into<String>) -> Self {
        Self {
            operations: vec![PatchOperation::SetParameter {
                id: id.into(),
                expr: expr.into(),
            }],
        }
    }

    pub fn set_parameters(
        operations: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            operations: operations
                .into_iter()
                .map(|(id, expr)| PatchOperation::SetParameter {
                    id: id.into(),
                    expr: expr.into(),
                })
                .collect(),
        }
    }

    pub fn set_feature_expr(
        feature_id: impl Into<String>,
        field: FeatureExprField,
        expr: impl Into<String>,
    ) -> Self {
        Self {
            operations: vec![PatchOperation::SetFeatureExpr {
                feature_id: feature_id.into(),
                field: field.as_str().to_string(),
                expr: expr.into(),
            }],
        }
    }

    pub fn apply_to_parameters(&self, graph: &mut ParamGraph) -> Result<()> {
        for operation in &self.operations {
            match operation {
                PatchOperation::SetParameter { id, expr } => {
                    graph.set_expr(id, expr.as_str()).map_err(|_| {
                        OpenCadError::validation(format!("unknown parameter '{id}'"))
                    })?;
                }
                PatchOperation::SetFeatureExpr { .. } => {}
            }
        }
        Ok(())
    }

    pub fn apply_to_features(&self, feature_nodes: &mut [FeatureNode]) -> Result<()> {
        for operation in &self.operations {
            let PatchOperation::SetFeatureExpr {
                feature_id,
                field,
                expr,
            } = operation
            else {
                continue;
            };
            let field = FeatureExprField::parse(field)?;
            let node = feature_nodes
                .iter_mut()
                .find(|node| node.id == *feature_id)
                .ok_or_else(|| {
                    OpenCadError::validation(format!("unknown feature '{feature_id}'"))
                })?;
            apply_feature_expr(node, field, expr)?;
        }
        Ok(())
    }

    pub fn apply_to_document(
        &self,
        parameters: &mut ParamGraph,
        feature_nodes: &mut [FeatureNode],
    ) -> Result<()> {
        self.apply_to_parameters(parameters)?;
        self.apply_to_features(feature_nodes)
    }
}

fn apply_feature_expr(
    node: &mut FeatureNode,
    field: FeatureExprField,
    expr: &str,
) -> Result<()> {
    match (&mut node.definition, field) {
        (FeatureDefinition::Extrude(extrude), FeatureExprField::LengthExpr) => {
            extrude.length_expr = Some(expr.to_string());
            Ok(())
        }
        (FeatureDefinition::Hole(hole), FeatureExprField::DepthExpr) => {
            hole.depth_expr = Some(expr.to_string());
            Ok(())
        }
        (FeatureDefinition::Fillet(fillet), FeatureExprField::RadiusExpr) => {
            fillet.radius_expr = Some(expr.to_string());
            Ok(())
        }
        (FeatureDefinition::Chamfer(chamfer), FeatureExprField::DistanceExpr) => {
            chamfer.distance_expr = Some(expr.to_string());
            Ok(())
        }
        (definition, field) => Err(OpenCadError::validation(format!(
            "feature '{}' ({}) does not support '{}'",
            node.id,
            definition.feature_type(),
            field.as_str()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencad_feature::{
        bracket_with_hole, bracket_with_top_chamfer, bracket_with_top_fillet, FeatureDefinition,
    };
    use opencad_graph::{bracket_parameters, evaluate_param_graph};

    #[test]
    fn set_parameter_patch_updates_graph() {
        let mut params = bracket_parameters();
        let patch = DesignPatch::set_parameter("param:width", "100 mm");
        patch.apply_to_parameters(&mut params).expect("patch");
        let values = evaluate_param_graph(&params).expect("eval");
        assert!((values["width"] - 0.1).abs() < 1e-9);
    }

    #[test]
    fn set_parameters_applies_multiple_values() {
        let mut params = bracket_parameters();
        let patch = DesignPatch::set_parameters([
            ("param:width", "100 mm"),
            ("param:thickness", "8 mm"),
        ]);
        patch.apply_to_parameters(&mut params).expect("patch");
        let values = evaluate_param_graph(&params).expect("eval");
        assert!((values["width"] - 0.1).abs() < 1e-9);
        assert!((values["thickness"] - 0.008).abs() < 1e-9);
    }

    #[test]
    fn set_feature_expr_updates_extrude_length_expr() {
        let part = bracket_with_hole().expect("model");
        let mut nodes: Vec<FeatureNode> = part.nodes.into_values().collect();
        let patch = DesignPatch::set_feature_expr(
            "feature:extrude_base",
            FeatureExprField::LengthExpr,
            "thickness * 2",
        );
        patch.apply_to_features(&mut nodes).expect("patch");
        let node = nodes
            .iter()
            .find(|node| node.id == "feature:extrude_base")
            .expect("extrude");
        let FeatureDefinition::Extrude(extrude) = &node.definition else {
            panic!("expected extrude");
        };
        assert_eq!(extrude.length_expr.as_deref(), Some("thickness * 2"));
    }

    #[test]
    fn set_feature_expr_rejects_unsupported_field() {
        let part = bracket_with_hole().expect("model");
        let mut nodes: Vec<FeatureNode> = part.nodes.into_values().collect();
        let patch = DesignPatch::set_feature_expr(
            "feature:hole_mount",
            FeatureExprField::LengthExpr,
            "thickness",
        );
        let err = patch.apply_to_features(&mut nodes).expect_err("field");
        assert!(err.to_string().contains("does not support"));
    }

    #[test]
    fn set_feature_expr_updates_fillet_radius_expr() {
        let part = bracket_with_top_fillet().expect("model");
        let mut nodes: Vec<FeatureNode> = part.nodes.into_values().collect();
        let patch = DesignPatch::set_feature_expr(
            "feature:fillet_top",
            FeatureExprField::RadiusExpr,
            "fillet_radius * 2",
        );
        patch.apply_to_features(&mut nodes).expect("patch");
        let node = nodes
            .iter()
            .find(|node| node.id == "feature:fillet_top")
            .expect("fillet");
        let FeatureDefinition::Fillet(fillet) = &node.definition else {
            panic!("expected fillet");
        };
        assert_eq!(fillet.radius_expr.as_deref(), Some("fillet_radius * 2"));
    }

    #[test]
    fn set_feature_expr_updates_chamfer_distance_expr() {
        let part = bracket_with_top_chamfer().expect("model");
        let mut nodes: Vec<FeatureNode> = part.nodes.into_values().collect();
        let patch = DesignPatch::set_feature_expr(
            "feature:chamfer_top",
            FeatureExprField::DistanceExpr,
            "chamfer_distance * 2",
        );
        patch.apply_to_features(&mut nodes).expect("patch");
        let node = nodes
            .iter()
            .find(|node| node.id == "feature:chamfer_top")
            .expect("chamfer");
        let FeatureDefinition::Chamfer(chamfer) = &node.definition else {
            panic!("expected chamfer");
        };
        assert_eq!(
            chamfer.distance_expr.as_deref(),
            Some("chamfer_distance * 2")
        );
    }
}
