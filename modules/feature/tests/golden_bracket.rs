//! Golden regression tests for the bracket-with-hole fixture (OCCT).

use opencad_feature::bracket_with_hole;
use opencad_graph::bracket_parameters;
use opencad_kernel_occt::OcctGeometryKernel;

mod support {
    include!("support/golden.rs");
}

use support::{assert_near, load_fixture, regen_volume_mass};

const FIXTURE: &str = include_str!("../../../fixtures/golden/bracket_with_hole.json");

#[test]
fn golden_bracket_default_volume_and_mass() {
    let fixture = load_fixture(FIXTURE);
    let case = fixture
        .cases
        .iter()
        .find(|case| case.id == "default")
        .expect("default case");
    let kernel = OcctGeometryKernel::new();
    let params = bracket_parameters();
    let mut model = bracket_with_hole().expect("model");
    let (volume_m3, mass_kg) =
        regen_volume_mass(&kernel, &mut model, &params, fixture.density_kg_per_m3);
    assert_near(volume_m3, case.volume_m3, case.volume_tol, "volume_m3");
    assert_near(mass_kg, case.mass_kg, case.mass_tol, "mass_kg");
}

#[test]
fn golden_bracket_patch_increases_volume() {
    let fixture = load_fixture(FIXTURE);
    let default = fixture
        .cases
        .iter()
        .find(|case| case.id == "default")
        .expect("default case");
    let patched = fixture
        .cases
        .iter()
        .find(|case| case.id == "width_100mm")
        .expect("width_100mm case");
    let patch = patched.patch.clone().expect("patch");

    let mut params = bracket_parameters();
    patch.apply_to_parameters(&mut params).expect("patch");

    let kernel = OcctGeometryKernel::new();
    let mut default_model = bracket_with_hole().expect("model");
    let (default_volume, _) = regen_volume_mass(
        &kernel,
        &mut default_model,
        &bracket_parameters(),
        fixture.density_kg_per_m3,
    );

    let mut patched_model = bracket_with_hole().expect("model");
    let (patched_volume, patched_mass) =
        regen_volume_mass(&kernel, &mut patched_model, &params, fixture.density_kg_per_m3);

    assert_near(
        patched_volume,
        patched.volume_m3,
        patched.volume_tol,
        "patched volume_m3",
    );
    assert_near(patched_mass, patched.mass_kg, patched.mass_tol, "patched mass_kg");

    let min_delta = patched.min_volume_delta_m3.expect("min delta");
    assert!(
        patched_volume - default_volume >= min_delta,
        "patched volume should exceed default: {patched_volume} - {default_volume}"
    );
    assert!(
        patched_volume > default.volume_m3,
        "patched volume should exceed default golden volume"
    );
}
