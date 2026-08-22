//! A small reusable linked feature plugin.
//!
//! This crate deliberately depends only on the serializable plugin/core/AI
//! contracts. It owns no document, filesystem, network, UI, or kernel state.

use opencad_ai::DesignPatch;
use opencad_core::{OpenCadError, Result};
use opencad_plugin_api::{
    FeaturePlugin, FeatureRequest, FeatureResult, PluginCapability, PluginKind, PluginManifest,
};

/// Stable ID used by the checked-in manifest and host registration.
pub const PLUGIN_ID: &str = "example.bracket-feature";
/// The serialized manifest shipped with this example.
pub const MANIFEST_JSON: &str = include_str!("../manifest.json");

/// A feature example that turns a parameter request into a DesignPatch.
///
/// The implementation is intentionally pure: the host owns document
/// validation, transactions, regeneration, and persistence.
#[derive(Debug, Clone)]
pub struct BracketFeaturePlugin {
    manifest: PluginManifest,
}

impl Default for BracketFeaturePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl BracketFeaturePlugin {
    /// Construct the example plugin with its stable manifest.
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest::new(
                PLUGIN_ID,
                "Bracket Feature Example",
                "0.1.0",
                PluginKind::Feature,
            )
            .with_capability(PluginCapability::FeaturePatch),
        }
    }
}

impl FeaturePlugin for BracketFeaturePlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn apply(&self, request: FeatureRequest) -> Result<FeatureResult> {
        let parameter_id = request
            .parameters
            .get("parameter_id")
            .ok_or_else(|| OpenCadError::validation("feature request requires 'parameter_id'"))?;
        let expr = request
            .parameters
            .get("expr")
            .ok_or_else(|| OpenCadError::validation("feature request requires 'expr'"))?;

        Ok(FeatureResult::new(DesignPatch::set_parameter(
            parameter_id,
            expr,
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use opencad_plugin_api::{PluginApiVersion, PluginManifest};

    fn request(expr: &str) -> FeatureRequest {
        FeatureRequest {
            feature_id: "feature:width_edit".into(),
            feature_type: PLUGIN_ID.into(),
            parameters: BTreeMap::from([
                ("expr".into(), expr.into()),
                ("parameter_id".into(), "param:width".into()),
            ]),
            inputs: Vec::new(),
        }
    }

    #[test]
    fn shipped_manifest_matches_source_manifest() {
        let shipped: PluginManifest = serde_json::from_str(MANIFEST_JSON).expect("manifest JSON");
        assert_eq!(&shipped, BracketFeaturePlugin::default().manifest());
        assert_eq!(shipped.id, PLUGIN_ID);
        assert_eq!(shipped.kind, PluginKind::Feature);
        assert!(shipped
            .capabilities
            .contains(&PluginCapability::FeaturePatch));
    }

    #[test]
    fn current_and_older_minor_api_versions_are_supported() {
        let manifest = BracketFeaturePlugin::default().manifest().clone();
        assert!(manifest
            .check_api_compatibility(PluginApiVersion::new(1, 0))
            .is_ok());
        assert!(manifest
            .check_api_compatibility(PluginApiVersion::new(1, 1))
            .is_ok());
    }

    #[test]
    fn future_and_major_api_versions_are_rejected() {
        let manifest = BracketFeaturePlugin::default().manifest().clone();
        let future_minor = manifest
            .clone()
            .with_api_version(PluginApiVersion::new(1, 1))
            .check_api_compatibility(PluginApiVersion::new(1, 0))
            .expect_err("future minor must be rejected");
        assert!(future_minor.to_string().contains("incompatible"));

        let major = manifest
            .with_api_version(PluginApiVersion::new(2, 0))
            .check_api_compatibility(PluginApiVersion::new(1, 0))
            .expect_err("major version must be rejected");
        assert!(major.to_string().contains("incompatible"));
    }

    #[test]
    fn feature_output_has_exact_deterministic_golden_bytes() {
        let result = BracketFeaturePlugin::default()
            .apply(request("100 mm"))
            .expect("feature result");
        assert_eq!(
            serde_json::to_string(&result).expect("result JSON"),
            r#"{"patch":{"operations":[{"type":"set_parameter","id":"param:width","expr":"100 mm"}]},"diagnostics":[]}"#
        );
        assert_eq!(
            serde_json::to_vec(&result.patch).expect("patch bytes"),
            br#"{"operations":[{"type":"set_parameter","id":"param:width","expr":"100 mm"}]}"#
        );
    }

    #[test]
    fn returned_error_does_not_change_reusable_plugin_behavior() {
        let plugin = BracketFeaturePlugin::default();
        let mut invalid = request("100 mm");
        invalid.parameters.remove("expr");
        let error = plugin.apply(invalid).expect_err("missing expr");
        assert!(error.to_string().contains("'expr'"));

        let valid = plugin.apply(request("100 mm")).expect("valid request");
        assert_eq!(valid.patch.operations.len(), 1);
    }
}
