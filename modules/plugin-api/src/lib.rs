//! Versioned, linked Rust contracts for MusubiCAD plugins.
//!
//! P4-001 intentionally defines data and traits only. A plugin receives
//! serializable request DTOs and either returns a [`opencad_ai::DesignPatch`]
//! or serializable output bytes/data. This crate does not provide dynamic
//! loading, registry ownership, document ownership, filesystem access,
//! network access, OCCT handles, or UI state.

pub mod exporter;
pub mod feature_plugin;
pub mod importer;
pub mod manifest;
pub mod registry;
pub mod ui_extension;

pub use exporter::{ExportRequest, ExportResult, ExporterPlugin};
pub use feature_plugin::{FeatureInput, FeaturePlugin, FeatureRequest, FeatureResult};
pub use importer::{ImportRequest, ImportResult, ImporterPlugin};
pub use manifest::{
    DiagnosticSeverity, PluginApiVersion, PluginCapability, PluginCapabilityPolicy,
    PluginDiagnostic, PluginError, PluginKind, PluginManifest, PluginResult, CURRENT_PLUGIN_API,
    PLUGIN_MANIFEST_SCHEMA,
};
pub use registry::PluginRegistry;
