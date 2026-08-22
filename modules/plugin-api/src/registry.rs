//! Deterministic, in-process plugin registration, capability policy, and
//! invocation.
//!
//! This registry stores trusted linked Rust implementations. It does not load
//! code or provide a loading/sandboxing boundary; host services decide when to
//! invoke a registered implementation.

use std::collections::BTreeMap;

use opencad_core::{OpenCadError, Result};

use crate::exporter::{ExportRequest, ExportResult, ExporterPlugin};
use crate::feature_plugin::{FeaturePlugin, FeatureRequest, FeatureResult};
use crate::importer::{ImportRequest, ImportResult, ImporterPlugin};
use crate::manifest::{
    PluginApiVersion, PluginCapability, PluginCapabilityPolicy, PluginKind, PluginManifest,
};

enum RegisteredPlugin {
    Feature(Box<dyn FeaturePlugin>),
    Importer(Box<dyn ImporterPlugin>),
    Exporter(Box<dyn ExporterPlugin>),
}

impl RegisteredPlugin {
    fn manifest(&self) -> &PluginManifest {
        match self {
            Self::Feature(plugin) => plugin.manifest(),
            Self::Importer(plugin) => plugin.manifest(),
            Self::Exporter(plugin) => plugin.manifest(),
        }
    }
}

/// Deterministic registry for trusted linked Rust plugin implementations.
///
/// Registration validates the manifest schema, host API compatibility, kind,
/// required capability, and host policy before storing a plugin. Entries are
/// keyed by manifest ID in a [`BTreeMap`], so [`Self::list`] and [`Self::ids`]
/// are independent of registration order. This type does not invoke plugins,
/// load code, or provide sandboxing; linked in-process plugin code is trusted.
pub struct PluginRegistry {
    host_api: PluginApiVersion,
    policy: PluginCapabilityPolicy,
    plugins: BTreeMap<String, RegisteredPlugin>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create a registry for the current plugin API with all declared data
    /// capabilities enabled.
    pub fn new() -> Self {
        Self {
            host_api: crate::manifest::CURRENT_PLUGIN_API,
            policy: PluginCapabilityPolicy::default(),
            plugins: BTreeMap::new(),
        }
    }

    /// Create a registry with an explicit host capability policy.
    pub fn with_policy(policy: PluginCapabilityPolicy) -> Self {
        Self {
            policy,
            ..Self::new()
        }
    }

    /// Create a registry that validates plugins against an explicit host API.
    pub fn with_host_api(host_api: PluginApiVersion) -> Self {
        Self {
            host_api,
            ..Self::new()
        }
    }

    pub fn host_api(&self) -> PluginApiVersion {
        self.host_api
    }

    pub fn policy(&self) -> &PluginCapabilityPolicy {
        &self.policy
    }

    /// Register a feature implementation after deterministic contract checks.
    pub fn register_feature<P>(&mut self, plugin: P) -> Result<()>
    where
        P: FeaturePlugin + 'static,
    {
        self.register(
            RegisteredPlugin::Feature(Box::new(plugin)),
            PluginKind::Feature,
        )
    }

    /// Register an importer implementation after deterministic contract checks.
    pub fn register_importer<P>(&mut self, plugin: P) -> Result<()>
    where
        P: ImporterPlugin + 'static,
    {
        self.register(
            RegisteredPlugin::Importer(Box::new(plugin)),
            PluginKind::Importer,
        )
    }

    /// Register an exporter implementation after deterministic contract checks.
    pub fn register_exporter<P>(&mut self, plugin: P) -> Result<()>
    where
        P: ExporterPlugin + 'static,
    {
        self.register(
            RegisteredPlugin::Exporter(Box::new(plugin)),
            PluginKind::Exporter,
        )
    }

    /// Return manifests in stable ID order. This calls only each plugin's
    /// metadata accessor; feature/importer/exporter operations are not run.
    pub fn list(&self) -> Vec<PluginManifest> {
        self.plugins
            .values()
            .map(|plugin| plugin.manifest().clone())
            .collect()
    }

    /// Return registered IDs in stable lexical order.
    pub fn ids(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Return one manifest by ID without invoking a plugin operation.
    pub fn manifest(&self, id: &str) -> Result<PluginManifest> {
        self.plugins
            .get(id)
            .map(|plugin| plugin.manifest().clone())
            .ok_or_else(|| OpenCadError::not_found(format!("plugin '{id}' is not registered")))
    }

    /// Invoke a registered feature plugin with an in-memory DTO request.
    pub fn invoke_feature(&self, id: &str, request: FeatureRequest) -> Result<FeatureResult> {
        match self.plugins.get(id) {
            Some(RegisteredPlugin::Feature(plugin)) => plugin.apply(request),
            Some(_) => Err(OpenCadError::validation(format!(
                "plugin '{id}' is not registered as a feature"
            ))),
            None => Err(OpenCadError::not_found(format!(
                "plugin '{id}' is not registered"
            ))),
        }
    }

    /// Invoke a registered importer plugin with caller-owned bytes.
    pub fn invoke_importer(&self, id: &str, request: ImportRequest) -> Result<ImportResult> {
        match self.plugins.get(id) {
            Some(RegisteredPlugin::Importer(plugin)) => plugin.import(request),
            Some(_) => Err(OpenCadError::validation(format!(
                "plugin '{id}' is not registered as an importer"
            ))),
            None => Err(OpenCadError::not_found(format!(
                "plugin '{id}' is not registered"
            ))),
        }
    }

    /// Invoke a registered exporter plugin with immutable serializable state.
    pub fn invoke_exporter(&self, id: &str, request: ExportRequest) -> Result<ExportResult> {
        match self.plugins.get(id) {
            Some(RegisteredPlugin::Exporter(plugin)) => plugin.export(request),
            Some(_) => Err(OpenCadError::validation(format!(
                "plugin '{id}' is not registered as an exporter"
            ))),
            None => Err(OpenCadError::not_found(format!(
                "plugin '{id}' is not registered"
            ))),
        }
    }

    fn register(&mut self, plugin: RegisteredPlugin, expected_kind: PluginKind) -> Result<()> {
        let manifest = plugin.manifest().clone();
        manifest.check_api_compatibility(self.host_api)?;
        if manifest.kind != expected_kind {
            return Err(OpenCadError::validation(format!(
                "plugin '{}' registered as {} but manifest declares {}",
                manifest.id,
                kind_name(expected_kind),
                kind_name(manifest.kind)
            )));
        }

        if self.plugins.contains_key(&manifest.id) {
            return Err(OpenCadError::validation(format!(
                "plugin registry duplicate ID '{}'",
                manifest.id
            )));
        }

        let required = PluginCapability::required_for(manifest.kind);
        if !manifest.capabilities.contains(&required) {
            return Err(OpenCadError::validation(format!(
                "plugin '{}' does not declare required capability '{}'",
                manifest.id,
                required.as_str()
            )));
        }

        for capability in &manifest.capabilities {
            if *capability != required {
                return Err(OpenCadError::validation(format!(
                    "plugin '{}' capability '{}' is not valid for kind '{}'",
                    manifest.id,
                    capability.as_str(),
                    kind_name(manifest.kind)
                )));
            }
            if !self.policy.allows(*capability) {
                return Err(OpenCadError::validation(format!(
                    "plugin '{}' capability '{}' is disallowed by host policy",
                    manifest.id,
                    capability.as_str()
                )));
            }
        }

        self.plugins.insert(manifest.id, plugin);
        Ok(())
    }
}

fn kind_name(kind: PluginKind) -> &'static str {
    match kind {
        PluginKind::Feature => "feature",
        PluginKind::Importer => "importer",
        PluginKind::Exporter => "exporter",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exporter::{ExportRequest, ExportResult};
    use crate::feature_plugin::{FeatureRequest, FeatureResult};
    use crate::importer::{ImportRequest, ImportResult};
    use opencad_ai::DesignPatch;

    struct FeatureStub {
        manifest: PluginManifest,
    }

    impl FeaturePlugin for FeatureStub {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }

        fn apply(&self, _request: FeatureRequest) -> Result<FeatureResult> {
            Ok(FeatureResult::new(DesignPatch::new(Vec::new())))
        }
    }

    struct ImporterStub {
        manifest: PluginManifest,
    }

    impl ImporterPlugin for ImporterStub {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }

        fn import(&self, _request: ImportRequest) -> Result<ImportResult> {
            Ok(ImportResult::new(DesignPatch::new(Vec::new())))
        }
    }

    struct ExporterStub {
        manifest: PluginManifest,
    }

    impl ExporterPlugin for ExporterStub {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }

        fn export(&self, _request: ExportRequest) -> Result<ExportResult> {
            Ok(ExportResult {
                format: "example".into(),
                media_type: "application/octet-stream".into(),
                data: Vec::new(),
                diagnostics: Vec::new(),
            })
        }
    }

    fn manifest(id: &str, kind: PluginKind, capability: PluginCapability) -> PluginManifest {
        PluginManifest::new(id, id, "0.1.0", kind).with_capability(capability)
    }

    #[test]
    fn listing_is_btree_ordered_across_all_three_plugin_kinds() {
        let mut registry = PluginRegistry::new();
        registry
            .register_exporter(ExporterStub {
                manifest: manifest(
                    "plugin:z",
                    PluginKind::Exporter,
                    PluginCapability::ExportBytes,
                ),
            })
            .expect("exporter");
        registry
            .register_feature(FeatureStub {
                manifest: manifest(
                    "plugin:a",
                    PluginKind::Feature,
                    PluginCapability::FeaturePatch,
                ),
            })
            .expect("feature");
        registry
            .register_importer(ImporterStub {
                manifest: manifest(
                    "plugin:m",
                    PluginKind::Importer,
                    PluginCapability::ImportPatch,
                ),
            })
            .expect("importer");

        assert_eq!(registry.ids(), ["plugin:a", "plugin:m", "plugin:z"]);
        assert_eq!(
            registry
                .list()
                .into_iter()
                .map(|manifest| manifest.id)
                .collect::<Vec<_>>(),
            ["plugin:a", "plugin:m", "plugin:z"]
        );
    }

    #[test]
    fn registration_rejects_undeclared_required_capability() {
        let mut registry = PluginRegistry::new();
        let error = registry
            .register_feature(FeatureStub {
                manifest: PluginManifest::new(
                    "plugin:missing",
                    "Missing",
                    "0.1.0",
                    PluginKind::Feature,
                ),
            })
            .expect_err("missing capability");
        assert!(error
            .to_string()
            .contains("does not declare required capability 'feature_patch'"));
        assert!(registry.is_empty());
    }

    #[test]
    fn registration_rejects_disallowed_capability() {
        let mut registry = PluginRegistry::with_policy(PluginCapabilityPolicy::new([
            PluginCapability::FeaturePatch,
        ]));
        let error = registry
            .register_importer(ImporterStub {
                manifest: manifest(
                    "plugin:blocked",
                    PluginKind::Importer,
                    PluginCapability::ImportPatch,
                ),
            })
            .expect_err("policy rejection");
        assert!(error
            .to_string()
            .contains("capability 'import_patch' is disallowed by host policy"));
        assert!(registry.is_empty());
    }

    #[test]
    fn registration_rejects_duplicate_ids_deterministically() {
        let mut registry = PluginRegistry::new();
        registry
            .register_feature(FeatureStub {
                manifest: manifest(
                    "plugin:same",
                    PluginKind::Feature,
                    PluginCapability::FeaturePatch,
                ),
            })
            .expect("first registration");
        let error = registry
            .register_feature(FeatureStub {
                manifest: manifest(
                    "plugin:same",
                    PluginKind::Feature,
                    PluginCapability::FeaturePatch,
                ),
            })
            .expect_err("duplicate ID");
        assert_eq!(
            error.to_string(),
            "validation failed: plugin registry duplicate ID 'plugin:same'"
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn registration_rejects_trait_kind_mismatch_and_extra_capability() {
        let mut registry = PluginRegistry::new();
        let kind_error = registry
            .register_feature(FeatureStub {
                manifest: manifest(
                    "plugin:wrong-kind",
                    PluginKind::Importer,
                    PluginCapability::ImportPatch,
                ),
            })
            .expect_err("kind mismatch");
        assert!(kind_error
            .to_string()
            .contains("registered as feature but manifest declares importer"));

        let extra_capability =
            PluginManifest::new("plugin:extra", "Extra", "0.1.0", PluginKind::Feature)
                .with_capabilities([
                    PluginCapability::FeaturePatch,
                    PluginCapability::ExportBytes,
                ]);
        let capability_error = registry
            .register_feature(FeatureStub {
                manifest: extra_capability,
            })
            .expect_err("kind capability mismatch");
        assert!(capability_error
            .to_string()
            .contains("capability 'export_bytes' is not valid for kind 'feature'"));
        assert!(registry.is_empty());
    }

    #[test]
    fn registration_validates_schema_and_api_before_storing() {
        let mut registry = PluginRegistry::with_host_api(PluginApiVersion::new(1, 1));
        let incompatible =
            PluginManifest::new("plugin:future", "Future", "0.1.0", PluginKind::Feature)
                .with_api_version(PluginApiVersion::new(2, 0))
                .with_capability(PluginCapability::FeaturePatch);
        let api_error = registry
            .register_feature(FeatureStub {
                manifest: incompatible,
            })
            .expect_err("API mismatch");
        assert!(api_error.to_string().contains("incompatible with host API"));

        let mut invalid_schema = manifest(
            "plugin:bad-schema",
            PluginKind::Feature,
            PluginCapability::FeaturePatch,
        );
        invalid_schema.schema = "musubicad.plugin-manifest.v2".into();
        let schema_error = registry
            .register_feature(FeatureStub {
                manifest: invalid_schema,
            })
            .expect_err("schema mismatch");
        assert!(schema_error
            .to_string()
            .contains("unsupported manifest schema"));
        assert!(registry.is_empty());
    }

    #[test]
    fn listing_manifests_remains_serializable() {
        let mut registry = PluginRegistry::new();
        registry
            .register_feature(FeatureStub {
                manifest: manifest(
                    "plugin:serializable",
                    PluginKind::Feature,
                    PluginCapability::FeaturePatch,
                ),
            })
            .expect("feature");
        let json = serde_json::to_string(&registry.list()).expect("listing JSON");
        let manifests: Vec<PluginManifest> =
            serde_json::from_str(&json).expect("listing round trip");
        assert_eq!(manifests, registry.list());
    }
}
