//! One-off helper to print golden fixture values. Run with:
//! `cargo test -p opencad-feature --test measure_fixture -- --ignored --nocapture`

use opencad_ai::{DesignPatch, FeatureExprField};
use opencad_feature::{
    bracket_with_hole, bracket_with_top_chamfer, bracket_with_top_fillet, FeatureRegistry,
};
use opencad_graph::bracket_parameters;
use opencad_geometry::GeometryKernel;
use opencad_kernel_occt::OcctGeometryKernel;

fn measure(label: &str, model: opencad_feature::PartModel, params: &opencad_graph::ParamGraph) {
    let kernel = OcctGeometryKernel::new();
    let registry = FeatureRegistry::with_defaults();
    let mut model = model;
    model.regenerate(&kernel, &registry, Some(params), None).expect("regen");
    let body = model.active_body().expect("body");
    let mass = kernel.mass_properties(body, 2700.0).expect("mass");
    println!("{label}: volume_m3={} mass_kg={}", mass.volume_m3, mass.mass_kg);
}

#[test]
#[ignore = "manual fixture calibration"]
fn print_golden_values() {
    let params = bracket_parameters();
    measure(
        "fillet_default",
        bracket_with_top_fillet().expect("fillet"),
        &params,
    );
    measure(
        "chamfer_default",
        bracket_with_top_chamfer().expect("chamfer"),
        &params,
    );
    measure("hole_ref", bracket_with_hole().expect("hole"), &params);

    let mut model = bracket_with_top_fillet().expect("fillet");
    let patch = DesignPatch::set_feature_expr(
        "feature:fillet_top",
        FeatureExprField::RadiusExpr,
        "fillet_radius * 2",
    );
    let mut nodes: Vec<_> = model.nodes.clone().into_values().collect();
    patch.apply_to_features(&mut nodes).expect("patch");
    for node in nodes {
        model.nodes.insert(node.id.clone(), node);
    }
    measure("fillet_radius_x2", model, &params);

    let mut chamfer_model = bracket_with_top_chamfer().expect("chamfer");
    let chamfer_patch = DesignPatch::set_feature_expr(
        "feature:chamfer_top",
        FeatureExprField::DistanceExpr,
        "chamfer_distance * 2",
    );
    let mut chamfer_nodes: Vec<_> = chamfer_model.nodes.clone().into_values().collect();
    chamfer_patch
        .apply_to_features(&mut chamfer_nodes)
        .expect("patch");
    for node in chamfer_nodes {
        chamfer_model.nodes.insert(node.id.clone(), node);
    }
    measure("chamfer_distance_x2", chamfer_model, &params);
}
