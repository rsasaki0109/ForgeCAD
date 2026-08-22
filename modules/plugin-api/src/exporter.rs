//! Linked Rust contract for serializable-state exporter plugins.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use opencad_core::Result;

use crate::manifest::{PluginDiagnostic, PluginManifest};

/// Export request containing immutable serializable design data.
///
/// The JSON value is a transport DTO; it is not an `OcadDocument` and gives a
/// plugin no ownership or mutation capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: String,
    pub state: Value,
}

/// Export result containing caller-owned bytes and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportResult {
    pub format: String,
    pub media_type: String,
    pub data: Vec<u8>,
    #[serde(default)]
    pub diagnostics: Vec<PluginDiagnostic>,
}

/// In-process, linked Rust exporter contract.
pub trait ExporterPlugin {
    fn manifest(&self) -> &PluginManifest;

    fn export(&self, request: ExportRequest) -> Result<ExportResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exporter_contract_round_trips_serializable_state_and_bytes() {
        let request = ExportRequest {
            format: "example.json".into(),
            state: serde_json::json!({"parameters": {"width": "80 mm"}}),
        };
        let result = ExportResult {
            format: "example.json".into(),
            media_type: "application/json".into(),
            data: br#"{"width":"80 mm"}"#.to_vec(),
            diagnostics: Vec::new(),
        };
        let request_json = serde_json::to_vec(&request).expect("request JSON");
        let result_json = serde_json::to_vec(&result).expect("result JSON");
        assert_eq!(
            serde_json::from_slice::<ExportRequest>(&request_json).unwrap(),
            request
        );
        assert_eq!(
            serde_json::from_slice::<ExportResult>(&result_json).unwrap(),
            result
        );
    }
}
