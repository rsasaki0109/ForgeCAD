//! Shared versioning, manifest, diagnostic, and error contracts.

use std::collections::BTreeSet;
use std::fmt;

use opencad_core::{OpenCadError, Result};
use serde::{Deserialize, Serialize};

/// Stable schema identifier for a serialized plugin manifest.
pub const PLUGIN_MANIFEST_SCHEMA: &str = "musubicad.plugin-manifest.v1";

/// The API version implemented by this workspace.
pub const CURRENT_PLUGIN_API: PluginApiVersion = PluginApiVersion::new(1, 0);

/// Major/minor version of the linked Rust plugin contract.
///
/// Compatibility is intentionally explicit: a plugin is accepted when its
/// major version matches the host and its minor version is no newer than the
/// host's. A major-version change is always rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PluginApiVersion {
    pub major: u16,
    pub minor: u16,
}

impl PluginApiVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Return whether a plugin built for this version can run on `host`.
    pub const fn is_compatible_with(self, host: Self) -> bool {
        self.major == host.major && self.minor <= host.minor
    }
}

/// The contract family implemented by a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Feature,
    Importer,
    Exporter,
}

/// Host capabilities that a plugin may declare.
///
/// The enum is intentionally limited to data-oriented operations. Filesystem,
/// network, UI, document ownership, and kernel access are not capabilities of
/// this API and therefore cannot be requested by a manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    FeaturePatch,
    ImportPatch,
    ExportBytes,
}

impl PluginCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FeaturePatch => "feature_patch",
            Self::ImportPatch => "import_patch",
            Self::ExportBytes => "export_bytes",
        }
    }

    pub const fn required_for(kind: PluginKind) -> Self {
        match kind {
            PluginKind::Feature => Self::FeaturePatch,
            PluginKind::Importer => Self::ImportPatch,
            PluginKind::Exporter => Self::ExportBytes,
        }
    }
}

/// Explicit host policy for the capabilities a registry may accept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilityPolicy {
    #[serde(default = "default_allowed_capabilities")]
    pub allowed: BTreeSet<PluginCapability>,
}

impl PluginCapabilityPolicy {
    pub fn new(capabilities: impl IntoIterator<Item = PluginCapability>) -> Self {
        Self {
            allowed: capabilities.into_iter().collect(),
        }
    }

    pub fn allows(&self, capability: PluginCapability) -> bool {
        self.allowed.contains(&capability)
    }
}

impl Default for PluginCapabilityPolicy {
    fn default() -> Self {
        Self::new([
            PluginCapability::FeaturePatch,
            PluginCapability::ImportPatch,
            PluginCapability::ExportBytes,
        ])
    }
}

fn default_allowed_capabilities() -> BTreeSet<PluginCapability> {
    PluginCapabilityPolicy::default().allowed
}

/// Serializable metadata exchanged before a plugin contract is invoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: PluginApiVersion,
    pub kind: PluginKind,
    /// Capabilities must be explicit for registry registration. The default
    /// keeps older v1 manifest JSON readable; such a manifest is rejected by
    /// the registry until it declares its kind capability.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub capabilities: BTreeSet<PluginCapability>,
}

impl PluginManifest {
    /// Construct a manifest for the current API version.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        kind: PluginKind,
    ) -> Self {
        Self {
            schema: PLUGIN_MANIFEST_SCHEMA.into(),
            id: id.into(),
            name: name.into(),
            version: version.into(),
            api_version: CURRENT_PLUGIN_API,
            kind,
            capabilities: BTreeSet::new(),
        }
    }

    /// Set the API version declared by a plugin.
    pub fn with_api_version(mut self, api_version: PluginApiVersion) -> Self {
        self.api_version = api_version;
        self
    }

    /// Declare the capabilities this plugin exposes to a host registry.
    pub fn with_capabilities(
        mut self,
        capabilities: impl IntoIterator<Item = PluginCapability>,
    ) -> Self {
        self.capabilities = capabilities.into_iter().collect();
        self
    }

    /// Add one capability declaration while retaining existing declarations.
    pub fn with_capability(mut self, capability: PluginCapability) -> Self {
        self.capabilities.insert(capability);
        self
    }

    /// Validate stable manifest fields before registration or invocation.
    pub fn validate(&self) -> Result<()> {
        if self.schema != PLUGIN_MANIFEST_SCHEMA {
            return Err(OpenCadError::validation(format!(
                "unsupported manifest schema '{}'; expected '{PLUGIN_MANIFEST_SCHEMA}'",
                self.schema
            )));
        }
        for (field, value) in [
            ("id", &self.id),
            ("name", &self.name),
            ("version", &self.version),
        ] {
            if value.trim().is_empty() {
                return Err(OpenCadError::validation(format!(
                    "manifest field '{field}' must not be empty"
                )));
            }
        }
        Ok(())
    }

    /// Validate this plugin against a host API version.
    pub fn check_api_compatibility(&self, host: PluginApiVersion) -> Result<()> {
        self.validate()?;
        if self.api_version.is_compatible_with(host) {
            Ok(())
        } else {
            Err(OpenCadError::validation(format!(
                "plugin API {}.{} is incompatible with host API {}.{}",
                self.api_version.major, self.api_version.minor, host.major, host.minor
            )))
        }
    }
}

/// Severity for a non-fatal plugin diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Serializable diagnostic returned alongside a contract result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
}

impl PluginDiagnostic {
    pub fn new(
        severity: DiagnosticSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Errors that can cross the plugin contract boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginError {
    InvalidManifest {
        message: String,
    },
    IncompatibleApi {
        plugin: PluginApiVersion,
        host: PluginApiVersion,
    },
    InvalidRequest {
        message: String,
    },
    UnsupportedFormat {
        format: String,
    },
    Execution {
        code: String,
        message: String,
    },
    Serialization {
        message: String,
    },
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidManifest { message } => write!(f, "invalid plugin manifest: {message}"),
            Self::IncompatibleApi { plugin, host } => write!(
                f,
                "plugin API {}.{} is incompatible with host API {}.{}",
                plugin.major, plugin.minor, host.major, host.minor
            ),
            Self::InvalidRequest { message } => write!(f, "invalid plugin request: {message}"),
            Self::UnsupportedFormat { format } => write!(f, "unsupported plugin format '{format}'"),
            Self::Execution { code, message } => {
                write!(f, "plugin execution error ({code}): {message}")
            }
            Self::Serialization { message } => write!(f, "plugin serialization error: {message}"),
        }
    }
}

impl std::error::Error for PluginError {}

impl From<serde_json::Error> for PluginError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization {
            message: error.to_string(),
        }
    }
}

/// Alias for the repository-standard fallible contract result.
pub type PluginResult<T> = opencad_core::Result<T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_serialization_is_stable_and_round_trips() {
        let manifest = PluginManifest::new(
            "example.bracket-importer",
            "Bracket Importer",
            "0.1.0",
            PluginKind::Importer,
        );
        let json = serde_json::to_string(&manifest).expect("manifest JSON");
        assert_eq!(
            json,
            r#"{"schema":"musubicad.plugin-manifest.v1","id":"example.bracket-importer","name":"Bracket Importer","version":"0.1.0","api_version":{"major":1,"minor":0},"kind":"importer"}"#
        );
        assert_eq!(
            serde_json::from_str::<PluginManifest>(&json).expect("manifest round trip"),
            manifest
        );
    }

    #[test]
    fn older_manifest_json_defaults_capabilities_without_changing_v1_bytes() {
        let json = r#"{"schema":"musubicad.plugin-manifest.v1","id":"example.feature","name":"Feature","version":"0.1.0","api_version":{"major":1,"minor":0},"kind":"feature"}"#;
        let manifest: PluginManifest = serde_json::from_str(json).expect("v1 manifest");
        assert!(manifest.capabilities.is_empty());
        assert_eq!(serde_json::to_string(&manifest).expect("v1 bytes"), json);
    }

    #[test]
    fn capability_policy_serialization_is_deterministic() {
        let policy = PluginCapabilityPolicy::new([
            PluginCapability::ExportBytes,
            PluginCapability::FeaturePatch,
        ]);
        let json = serde_json::to_string(&policy).expect("policy JSON");
        assert_eq!(json, r#"{"allowed":["feature_patch","export_bytes"]}"#);
        assert_eq!(
            serde_json::from_str::<PluginCapabilityPolicy>(&json).expect("policy round trip"),
            policy
        );
    }

    #[test]
    fn compatibility_accepts_older_minor_and_rejects_future_or_major() {
        let older = PluginApiVersion::new(1, 0);
        let host = PluginApiVersion::new(1, 2);
        assert!(older.is_compatible_with(host));
        assert!(!PluginApiVersion::new(1, 3).is_compatible_with(host));
        assert!(!PluginApiVersion::new(2, 0).is_compatible_with(host));

        let manifest =
            PluginManifest::new("example.feature", "Feature", "0.1.0", PluginKind::Feature);
        assert!(manifest.check_api_compatibility(host).is_ok());
        let incompatible = manifest
            .with_api_version(PluginApiVersion::new(2, 0))
            .check_api_compatibility(host)
            .expect_err("major version must be rejected");
        assert!(incompatible
            .to_string()
            .contains("incompatible with host API"));
    }

    #[test]
    fn invalid_manifest_is_a_serializable_error() {
        let manifest = PluginManifest::new("", "Feature", "0.1.0", PluginKind::Feature);
        let error = manifest.validate().expect_err("empty ID");
        assert!(error.to_string().contains("manifest field 'id'"));

        let diagnostic = PluginError::InvalidManifest {
            message: "empty id".into(),
        };
        let bytes = serde_json::to_vec(&diagnostic).expect("error JSON");
        let decoded: PluginError = serde_json::from_slice(&bytes).expect("error round trip");
        assert_eq!(decoded, diagnostic);
    }
}
