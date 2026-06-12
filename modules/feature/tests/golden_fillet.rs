//! Golden regression tests for the bracket-with-top-fillet fixture (OCCT).

use opencad_feature::bracket_with_top_fillet;
use opencad_graph::bracket_parameters;
use opencad_kernel_occt::OcctGeometryKernel;

mod support {
    include!("support/golden.rs");
}

use support::{apply_patch, assert_near, load_fixture, regen_volume_mass};

const FIXTURE: &str = include_str!("../../../fixtures/golden/bracket_with_top_fillet.json");

#[test]
fn golden_fillet_default_volume_and_mass() {
    let fixture = load_fixture(FIXTURE);
    let case = fixture
        .cases
        .iter()
        .find(|case| case.id == "default")
        .expect("default case");

    let kernel = OcctGeometryKernel::new();
    let params = bracket_parameters();
    let mut model = bracket_with_top_fillet().expect("model");
    let (volume_m3, mass_kg) =
        regen_volume_mass(&kernel, &mut model, &params, fixture.density_kg_per_m3);
    assert_near(volume_m3, case.volume_m3, case.volume_tol, "volume_m3");
    assert_near(mass_kg, case.mass_kg, case.mass_tol, "mass_kg");
}

#[test]
fn golden_fillet_radius_patch_reduces_volume() {
    let fixture = load_fixture(FIXTURE);
    let default = fixture
        .cases
        .iter()
        .find(|case| case.id == "default")
        .expect("default case");
    let patched_case = fixture
        .cases
        .iter()
        .find(|case| case.id == "fillet_radius_x2")
        .expect("fillet_radius_x2 case");
    let patch = patched_case.patch.clone().expect("patch");

    let kernel = OcctGeometryKernel::new();
    let params = bracket_parameters();
    let mut default_model = bracket_with_top_fillet().expect("model");
    let (default_volume, _) = regen_volume_mass(
        &kernel,
        &mut default_model,
        &params,
        fixture.density_kg_per_m3,
    );

    let mut patched_model = bracket_with_top_fillet().expect("model");
    let mut patched_params = bracket_parameters();
    apply_patch(&mut patched_model, &mut patched_params, &patch);
    let (patched_volume, patched_mass) = regen_volume_mass(
        &kernel,
        &mut patched_model,
        &patched_params,
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
        "larger fillet should remove more material: {default_volume} - {patched_volume}"
    );
    assert!(patched_volume < default.volume_m3);
}
