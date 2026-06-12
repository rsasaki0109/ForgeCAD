//! Hole feature: extruded profile cut (Task-093+).

use serde::{Deserialize, Serialize};

use opencad_core::{OpenCadError, Result};
use opencad_geometry::ExtrudeExtent;

use crate::extrude::{ExtrudeFeature, ExtrudeFeatureExecutor};
use crate::feature::{
    Feature, FeatureDefinition, FeatureNode, FeatureOutput, RegenContext,
};

/// Hole feature parameters (sketch circle/profile cut into a body).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoleFeature {
    pub sketch_feature: String,
    pub profile_ref: String,
    pub depth: ExtrudeExtent,
    pub target_feature: String,
    /// Parametric depth expression resolved before regeneration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_expr: Option<String>,
}

/// Hole executor implemented as an extrude cut.
#[derive(Debug, Default)]
pub struct HoleFeatureExecutor;

impl Feature for HoleFeatureExecutor {
    fn feature_type(&self) -> &'static str {
        "hole"
    }

    fn execute(&self, node: &FeatureNode, ctx: &dyn RegenContext) -> Result<FeatureOutput> {
        let FeatureDefinition::Hole(def) = &node.definition else {
            return Err(OpenCadError::validation(format!(
                "expected hole feature, got {}",
                node.definition.feature_type()
            )));
        };

        let extrude_node = FeatureNode::new(
            node.id.clone(),
            node.name.clone(),
            FeatureDefinition::Extrude(ExtrudeFeature {
                sketch_feature: def.sketch_feature.clone(),
                profile_ref: def.profile_ref.clone(),
                extent: def.depth.clone(),
                operation: opencad_geometry::ExtrudeOperation::Cut,
                length_expr: def.depth_expr.clone(),
                target_feature: Some(def.target_feature.clone()),
            }),
        );
        ExtrudeFeatureExecutor.execute(&extrude_node, ctx)
    }
}

impl HoleFeature {
    pub fn through(
        sketch_feature: impl Into<String>,
        profile_ref: impl Into<String>,
        depth: ExtrudeExtent,
        target_feature: impl Into<String>,
    ) -> Self {
        Self {
            sketch_feature: sketch_feature.into(),
            profile_ref: profile_ref.into(),
            depth,
            target_feature: target_feature.into(),
            depth_expr: None,
        }
    }
}
