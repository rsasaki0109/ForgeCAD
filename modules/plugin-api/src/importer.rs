//! Linked Rust contract for byte-oriented importer plugins.

use opencad_ai::DesignPatch;
use opencad_core::Result;
use serde::{Deserialize, Serialize};

use crate::manifest::{PluginDiagnostic, PluginManifest};

/// Import request containing caller-owned bytes, never a filesystem handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRequest {
    pub format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    pub data: Vec<u8>,
}

/// Import result expressed as a validated DesignPatch plus diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportResult {
    pub patch: DesignPatch,
    #[serde(default)]
    pub diagnostics: Vec<PluginDiagnostic>,
}

impl ImportResult {
    pub fn new(patch: DesignPatch) -> Self {
        Self {
            patch,
            diagnostics: Vec::new(),
        }
    }
}

/// In-process, linked Rust importer contract.
pub trait ImporterPlugin {
    fn manifest(&self) -> &PluginManifest;

    fn import(&self, request: ImportRequest) -> Result<ImportResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importer_contract_is_byte_and_patch_serializable() {
        let request = ImportRequest {
            format: "example.mesh".into(),
            source_name: Some("bracket.mesh".into()),
            data: vec![0, 1, 2, 255],
        };
        let result = ImportResult::new(DesignPatch::set_parameter("param:width", "80 mm"));
        let request_json = serde_json::to_vec(&request).expect("request JSON");
        let result_json = serde_json::to_vec(&result).expect("result JSON");
        assert_eq!(
            serde_json::from_slice::<ImportRequest>(&request_json).unwrap(),
            request
        );
        assert_eq!(
            serde_json::from_slice::<ImportResult>(&result_json).unwrap(),
            result
        );
    }
}
