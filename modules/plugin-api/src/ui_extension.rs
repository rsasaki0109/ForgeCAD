//! UI state is intentionally not part of the P4-001 plugin contract.
//!
//! Plugins exchange serializable design data through the feature/importer/
//! exporter modules. Viewport, camera, selection, and other UI-owned state
//! remain outside this crate.
