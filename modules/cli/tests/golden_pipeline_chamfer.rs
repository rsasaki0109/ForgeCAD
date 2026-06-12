//! Golden pipeline tests for bracket-with-top-chamfer (read → patch → regen).

use opencad_core::{DocumentId, DocumentMetadata};
use opencad_feature::{bracket_with_top_chamfer, FeatureRegistry};
use opencad_file::{apply_patch_to_document, read_ocad, write_expanded_dir, OcadDocument};
use opencad_geometry::GeometryKernel;
use opencad_graph::bracket_parameters;
use opencad_kernel_occt::OcctGeometryKernel;
use tempfile::tempdir;

mod support {
    include!("support/golden.rs");
}

use support::{assert_near, load_fixture};

const FIXTURE: &str = include_str!("../../../fixtures/golden/bracket_with_top_chamfer.json");

fn write_chamfer_fixture(path: &std::path::Path) {
    let part = bracket_with_top_chamfer().expect("model");
    let metadata = DocumentMetadata::new(
        DocumentId::new("doc:bracket_chamfer").expect("id"),
        "Bracket with Top Chamfer",
    );
    let mut doc = OcadDocument::from_part_model(metadata, &part);
    doc.parameters = bracket_parameters();
    write_expanded_dir(path, &doc).expect("write");
}

fn regen_volume_mass(path: &str, density_kg_per_m3: f64) -> (f64, f64, usize) {
    let doc = read_ocad(path).expect("read");
    let params = doc.parameters.clone();
    let mut model = doc.into_part_model();
    let kernel = OcctGeometryKernel::new();
    let registry = FeatureRegistry::with_defaults();
    let report = model
        .regenerate(&kernel, &registry, Some(&params), None)
        .expect("regen");
    let body = model.active_body().expect("body");
    let mass = kernel
        .mass_properties(body, density_kg_per_m3)
        .expect("mass");
    (mass.volume_m3, mass.mass_kg, report.regenerated.len())
}

fn apply_patch_and_regen(
    path: &str,
    patch: &opencad_ai::DesignPatch,
    density_kg_per_m3: f64,
) -> (f64, f64) {
    let mut doc = read_ocad(path).expect("read");
    apply_patch_to_document(&mut doc, patch).expect("patch");
    write_expanded_dir(path, &doc).expect("write");
    let (volume_m3, mass_kg, _) = regen_volume_mass(path, density_kg_per_m3);
    (volume_m3, mass_kg)
}

#[test]
fn golden_pipeline_chamfer_default_regen_matches_fixture() {
    let fixture = load_fixture(FIXTURE);
    let case = fixture
        .cases
        .iter()
        .find(|case| case.id == "default")
        .expect("default case");

    let dir = tempdir().expect("tempdir");
    let doc_path = dir.path().join("bracket_chamfer.ocad.d");
    write_chamfer_fixture(&doc_path);

    let (volume_m3, mass_kg, feature_count) =
        regen_volume_mass(doc_path.to_str().expect("path"), fixture.density_kg_per_m3);
    assert_eq!(feature_count, 5);
    assert_near(volume_m3, case.volume_m3, case.volume_tol, "volume_m3");
    assert_near(mass_kg, case.mass_kg, case.mass_tol, "mass_kg");
}

#[test]
fn golden_pipeline_chamfer_distance_patch_reduces_volume() {
    let fixture = load_fixture(FIXTURE);
    let default = fixture
        .cases
        .iter()
        .find(|case| case.id == "default")
        .expect("default case");
    let patched_case = fixture
        .cases
        .iter()
        .find(|case| case.id == "chamfer_distance_x2")
        .expect("chamfer_distance_x2 case");
    let patch = patched_case.patch.clone().expect("patch");

    let dir = tempdir().expect("tempdir");
    let doc_path = dir.path().join("bracket_chamfer.ocad.d");
    write_chamfer_fixture(&doc_path);

    let (default_volume, _, _) =
        regen_volume_mass(doc_path.to_str().expect("path"), fixture.density_kg_per_m3);

    let (patched_volume, patched_mass) = apply_patch_and_regen(
        doc_path.to_str().expect("path"),
        &patch,
        fixture.density_kg_per_m3,
    );

    assert_near(
        patched_volume,
        patched_case.volume_m3,
        patched_case.volume_tol,
        "patched volume_m3",
    );
    assert_near(
        patched_mass,
        patched_case.mass_kg,
        patched_case.mass_tol,
        "patched mass_kg",
    );

    let min_delta = patched_case.min_volume_delta_m3.expect("min delta");
    assert!(
        default_volume - patched_volume >= min_delta,
        "larger chamfer should remove more material: {default_volume} - {patched_volume}"
    );
    assert!(patched_volume < default.volume_m3);
}
