//! Headless acceptance path for the shared desktop backend.

#![cfg(feature = "occt")]

use std::fs;
use std::path::{Path, PathBuf};

use opencad_desktop::run_desktop_smoke;
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn bracket_backend_smoke_open_preview_edit_regen_pick_export() {
    let source = workspace_root().join("examples/bracket.ocad.d");
    let temp = tempdir().expect("tempdir");
    let work_dir = temp.path().join("desktop-smoke.ocad.d");

    let summary = run_desktop_smoke(&source, &work_dir).expect("desktop smoke");
    let summary_json = serde_json::to_value(&summary).expect("serializable smoke summary");

    assert_eq!(summary.document_name, "Bracket with Mounting Hole");
    assert_eq!(summary.document_kind, opencad_core::DocumentKind::Part);
    assert!(summary.copied_files >= 1);
    assert!(summary.source_unchanged);
    assert!(summary.preview_triangles > 0);
    assert!(summary.preview_vertices > 0);
    assert_eq!(summary.edited_parameter_id, "param:width");
    assert!((summary.width_before_mm - 80.0).abs() < 1e-9);
    assert!((summary.width_after_mm - 100.0).abs() < 1e-9);
    assert!(summary.regenerated_features >= 4);
    assert!(summary.regeneration_triangles > 0);
    assert!(summary.pick_highlight_segments > 0);
    assert!(summary.exported_triangles > 0);
    assert!(Path::new(&summary.export_path).is_file());
    assert_eq!(summary_json["source_unchanged"], true);
    assert!(fs::metadata(&work_dir).expect("workdir").is_dir());
}

#[test]
fn smoke_refuses_to_overwrite_existing_work_directory() {
    let source = workspace_root().join("examples/bracket.ocad.d");
    let temp = tempdir().expect("tempdir");
    let work_dir = temp.path().join("existing.ocad.d");
    fs::create_dir(&work_dir).expect("existing workdir");

    let error = run_desktop_smoke(&source, &work_dir).expect_err("existing workdir");
    assert!(error.to_string().contains("already exists"));
}
