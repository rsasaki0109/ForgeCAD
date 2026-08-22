//! Linked Rust contract for custom parametric feature plugins.

use std::collections::BTreeMap;

use opencad_ai::DesignPatch;
use opencad_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::manifest::{PluginDiagnostic, PluginManifest};

/// A serializable input selected by semantic role rather than kernel handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureInput {
    pub role: String,
    pub value: Value,
}

/// Request passed to a feature plugin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureRequest {
    pub feature_id: String,
    pub feature_type: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, String>,
    #[serde(default)]
    pub inputs: Vec<FeatureInput>,
}

/// Result returned by a feature plugin.
///
/// The host applies and validates the patch; a plugin never receives document
/// ownership and cannot mutate the Design Graph directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureResult {
    pub patch: DesignPatch,
    #[serde(default)]
    pub diagnostics: Vec<PluginDiagnostic>,
}

impl FeatureResult {
    pub fn new(patch: DesignPatch) -> Self {
        Self {
            patch,
            diagnostics: Vec::new(),
        }
    }
}

/// In-process, linked Rust feature contract.
pub trait FeaturePlugin {
    fn manifest(&self) -> &PluginManifest;

    fn apply(&self, request: FeatureRequest) -> Result<FeatureResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_request_and_patch_result_round_trip() {
        let request = FeatureRequest {
            feature_id: "feature:rib".into(),
            feature_type: "example.rib".into(),
            parameters: BTreeMap::from([(String::from("length"), String::from("12 mm"))]),
            inputs: vec![FeatureInput {
                role: "profile".into(),
                value: serde_json::json!({"sketch_id": "sketch:rib"}),
            }],
        };
        let result = FeatureResult::new(DesignPatch::set_parameter("param:length", "12 mm"));
        let request_json = serde_json::to_vec(&request).expect("request JSON");
        let result_json = serde_json::to_vec(&result).expect("result JSON");
        assert_eq!(
            serde_json::from_slice::<FeatureRequest>(&request_json).unwrap(),
            request
        );
        assert_eq!(
            serde_json::from_slice::<FeatureResult>(&result_json).unwrap(),
            result
        );
    }
}
