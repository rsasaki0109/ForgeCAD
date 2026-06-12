//! Golden regression tests for the `.ocad` read → patch → regen pipeline.

use opencad_core::{DocumentId, DocumentMetadata};
use opencad_feature::{bracket_with_hole, FeatureRegistry};
use opencad_file::{read_ocad, write_expanded_dir, OcadDocument};
use opencad_geometry::GeometryKernel;
use opencad_graph::bracket_parameters;
use opencad_kernel_occt::OcctGeometryKernel;
use tempfile::tempdir;

mod support {
    include!("support/golden.rs");
}

use support::{assert_near, load_fixture};

const FIXTURE: &str = include_str!("../../../fixtures/golden/bracket_with_hole.json");

fn write_bracket_fixture(path: &std::path::Path) {
    let part = bracket_with_hole().expect("model");
    let metadata = DocumentMetadata::new(
        DocumentId::new("doc:bracket_001").expect("id"),
        "Bracket with Mounting Hole",
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

#[test]
fn golden_pipeline_default_regen_matches_fixture() {
    let fixture = load_fixture(FIXTURE);
    let case = fixture
        .cases
        .iter()
        .find(|case| case.id == "default")
        .expect("default case");

    let dir = tempdir().expect("tempdir");
    let doc_path = dir.path().join("bracket.ocad.d");
    write_bracket_fixture(&doc_path);

    let (volume_m3, mass_kg, feature_count) =
        regen_volume_mass(doc_path.to_str().expect("path"), fixture.density_kg_per_m3);
    assert_eq!(feature_count, 4);
    assert_near(volume_m3, case.volume_m3, case.volume_tol, "volume_m3");
    assert_near(mass_kg, case.mass_kg, case.mass_tol, "mass_kg");
}

#[test]
fn golden_pipeline_patch_regen_matches_fixture() {
    let fixture = load_fixture(FIXTURE);
    let default = fixture
        .cases
        .iter()
        .find(|case| case.id == "default")
        .expect("default case");
    let patched_case = fixture
        .cases
        .iter()
        .find(|case| case.id == "width_100mm")
        .expect("width_100mm case");
    let patch = patched_case.patch.clone().expect("patch");

    let dir = tempdir().expect("tempdir");
    let doc_path = dir.path().join("bracket.ocad.d");
    write_bracket_fixture(&doc_path);

    let (default_volume, _, _) =
        regen_volume_mass(doc_path.to_str().expect("path"), fixture.density_kg_per_m3);

    let mut doc = read_ocad(&doc_path).expect("read");
    patch
        .apply_to_parameters(&mut doc.parameters)
        .expect("patch");
    write_expanded_dir(&doc_path, &doc).expect("write");

    let values = opencad_graph::evaluate_param_graph(&doc.parameters).expect("eval");
    assert!((values["width"] - 0.1).abs() < 1e-9);

    let (patched_volume, patched_mass, _) =
        regen_volume_mass(doc_path.to_str().expect("path"), fixture.density_kg_per_m3);
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
    assert!(patched_volume - default_volume >= min_delta);
    assert!(patched_volume > default.volume_m3);
}
