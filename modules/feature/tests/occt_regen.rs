//! OCCT-backed feature regeneration integration tests.

use opencad_core::Length;
use opencad_feature::{bracket_base_plate, bracket_with_hole, bracket_with_top_chamfer, bracket_with_top_fillet, profile_to_solved, FeatureRegistry};
use opencad_graph::bracket_parameters;
use opencad_geometry::{build_src_to_post_map, ExtrudeExtent, ExtrudeOperation, GeometryKernel};
use opencad_kernel_occt::OcctGeometryKernel;

#[test]
fn occt_direct_extrude_matches_expected_volume() {
    let model = bracket_base_plate().expect("model");
    let sketch = model.sketches.get("sketch:base").expect("sketch");
    let solved = profile_to_solved(sketch, "sketch:base/profile:outer").expect("solved");
    let kernel = OcctGeometryKernel::new();
    let wire = kernel.make_wire_from_sketch(&solved).expect("wire");
    let body = kernel
        .extrude(
            wire,
            ExtrudeExtent::Distance {
                length: Length::from_meters(0.006),
            },
            ExtrudeOperation::NewBody,
            None,
        )
        .expect("extrude");
    let mass = kernel.mass_properties(&body, 2700.0).expect("mass");
    let expected = 0.08 * 0.06 * 0.006;
    assert!(
        (mass.volume_m3 - expected).abs() < 1e-8,
        "points={:?} volume={} expected={}",
        solved.points,
        mass.volume_m3,
        expected
    );
}

#[test]
fn occt_regenerates_bracket_plate_volume() {
    let mut model = bracket_base_plate().expect("model");
    let kernel = OcctGeometryKernel::new();
    let registry = FeatureRegistry::with_defaults();
    model
        .regenerate(&kernel, &registry, None, None)
        .expect("regen");

    let body = model.active_body().expect("body");
    let mass = kernel.mass_properties(body, 2700.0).expect("mass");
    let expected = 0.08 * 0.06 * 0.006;
    assert!(
        (mass.volume_m3 - expected).abs() < 1e-8,
        "volume={} expected={}",
        mass.volume_m3,
        expected
    );
    assert!(mass.mass_kg > 0.0);
}

#[test]
fn occt_regenerates_bracket_with_hole_reduces_volume() {
    let mut model = bracket_with_hole().expect("model");
    let kernel = OcctGeometryKernel::new();
    let registry = FeatureRegistry::with_defaults();
    let params = bracket_parameters();
    model
        .regenerate(&kernel, &registry, Some(&params), None)
        .expect("regen");

    let body = model.active_body().expect("body");
    let mass = kernel.mass_properties(body, 2700.0).expect("mass");
    let plate_volume = 0.08 * 0.06 * 0.006;
    let hole_radius = 0.005;
    let hole_volume = std::f64::consts::PI * hole_radius * hole_radius * 0.006;
    let expected = plate_volume - hole_volume;
    assert!(
        mass.volume_m3 < plate_volume,
        "hole should reduce volume: {} vs plate {}",
        mass.volume_m3,
        plate_volume
    );
    assert!(
        (mass.volume_m3 - expected).abs() < 1e-7,
        "volume={} expected={}",
        mass.volume_m3,
        expected
    );
}

#[test]
fn occt_top_fillet_reduces_volume() {
    let mut model = bracket_with_top_fillet().expect("model");
    let kernel = OcctGeometryKernel::new();
    let registry = FeatureRegistry::with_defaults();
    let params = bracket_parameters();
    model
        .regenerate(&kernel, &registry, Some(&params), None)
        .expect("regen");

    let body = model.active_body().expect("body");
    let mass = kernel.mass_properties(body, 2700.0).expect("mass");

    let mut without_fillet = bracket_with_hole().expect("model");
    without_fillet
        .regenerate(&kernel, &registry, Some(&params), None)
        .expect("regen");
    let base_body = without_fillet.active_body().expect("body");
    let base_mass = kernel
        .mass_properties(base_body, 2700.0)
        .expect("base mass");

    assert!(
        mass.volume_m3 < base_mass.volume_m3,
        "fillet should reduce volume: {} vs {}",
        mass.volume_m3,
        base_mass.volume_m3
    );
    assert!(mass.mass_kg > 0.0);
}

#[test]
fn occt_regen_composes_boolean_and_fillet_history() {
    let mut model = bracket_with_top_fillet().expect("model");
    let kernel = OcctGeometryKernel::new();
    let registry = FeatureRegistry::with_defaults();
    let params = bracket_parameters();
    let report = model
        .regenerate(&kernel, &registry, Some(&params), None)
        .expect("regen");
    let body = model.active_body().expect("body");
    let final_only = kernel.face_derivation_history(body);

    assert!(
        report.face_history.len() > final_only.len(),
        "composed history should include boolean + fillet steps"
    );

    let composed_map = build_src_to_post_map(&report.face_history);
    let final_map = build_src_to_post_map(&final_only);
    assert!(
        composed_map.len() > final_map.len(),
        "composed map should track more ancestor face ids"
    );
}

#[test]
fn occt_top_chamfer_reduces_volume() {
    let mut model = bracket_with_top_chamfer().expect("model");
    let kernel = OcctGeometryKernel::new();
    let registry = FeatureRegistry::with_defaults();
    let params = bracket_parameters();
    model
        .regenerate(&kernel, &registry, Some(&params), None)
        .expect("regen");

    let body = model.active_body().expect("body");
    let mass = kernel.mass_properties(body, 2700.0).expect("mass");

    let mut without_chamfer = bracket_with_hole().expect("model");
    without_chamfer
        .regenerate(&kernel, &registry, Some(&params), None)
        .expect("regen");
    let base_body = without_chamfer.active_body().expect("body");
    let base_mass = kernel
        .mass_properties(base_body, 2700.0)
        .expect("base mass");

    assert!(
        mass.volume_m3 < base_mass.volume_m3,
        "chamfer should reduce volume: {} vs {}",
        mass.volume_m3,
        base_mass.volume_m3
    );
    assert!(mass.mass_kg > 0.0);
}

#[test]
fn occt_fillet_on_face_ref_matches_top_perimeter() {
    use opencad_core::TopoRefId;
    use opencad_feature::{FeatureDefinition, FeatureNode, FilletFeature};
    use opencad_geometry::TopoRef;

    let params = bracket_parameters();
    let kernel = OcctGeometryKernel::new();
    let registry = FeatureRegistry::with_defaults();

    let mut face_ref_model = bracket_with_hole().expect("model");
    face_ref_model
        .add_node(FeatureNode::new(
            "feature:fillet_top",
            "Top Fillet",
            FeatureDefinition::Fillet(FilletFeature::on_face_ref(
                "feature:hole_mount",
                "ref:face:bracket_top",
                Length::from_meters(0.001),
                Some("fillet_radius".into()),
            )),
        ))
        .expect("node");
    face_ref_model
        .add_dependency("feature:hole_mount", "feature:fillet_top")
        .expect("dep");

    let mut baseline_model = bracket_with_top_fillet().expect("model");
    baseline_model
        .regenerate(&kernel, &registry, Some(&params), None)
        .expect("regen");
    let baseline_mass = kernel
        .mass_properties(baseline_model.active_body().expect("body"), 2700.0)
        .expect("mass");

    let semantic_refs = vec![TopoRef::face(
        TopoRefId::new("ref:face:bracket_top").expect("id"),
        "feature:extrude_base",
        "top",
    )];

    face_ref_model
        .regenerate(&kernel, &registry, Some(&params), Some(&semantic_refs))
        .expect("regen");
    let face_ref_mass = kernel
        .mass_properties(face_ref_model.active_body().expect("body"), 2700.0)
        .expect("mass");

    assert!(
        (face_ref_mass.volume_m3 - baseline_mass.volume_m3).abs() < 1e-8,
        "face_ref fillet volume {} should match top perimeter {}",
        face_ref_mass.volume_m3,
        baseline_mass.volume_m3
    );
}
