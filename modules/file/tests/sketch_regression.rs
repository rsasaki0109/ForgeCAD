//! Deterministic regression coverage for serialized sketch constraints.

use std::fs;
use std::path::{Path, PathBuf};

use opencad_file::{read_expanded_dir, validate_expanded_dir, write_expanded_dir};
use opencad_sketch::{parse_length_expr, solve_sketch, Coord, Sketch, SketchEntity, SolveState};
use opencad_solver::{SolveStatus, SolverOptions};

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("examples/sketch_constraints_regression.ocad.d")
}

fn sketch<'a>(doc: &'a opencad_file::OcadDocument, id: &str) -> &'a Sketch {
    doc.sketches
        .iter()
        .find(|sketch| sketch.id.as_str() == id)
        .unwrap_or_else(|| panic!("missing sketch '{id}'"))
}

fn point_coordinates(sketch: &Sketch) -> Vec<(String, [f64; 2])> {
    sketch
        .entities
        .iter()
        .filter_map(|entity| {
            let SketchEntity::Point(point) = entity else {
                return None;
            };
            let Coord::Literal(x) = point.x else {
                panic!("point '{}' has a non-literal x", point.base.id);
            };
            let Coord::Literal(y) = point.y else {
                panic!("point '{}' has a non-literal y", point.base.id);
            };
            Some((point.base.id.as_str().to_string(), [x, y]))
        })
        .collect()
}

fn solve_once(source: &Sketch) -> (Vec<(String, [f64; 2])>, SolveStatus, SolveState) {
    let mut sketch = source.clone();
    let status = solve_sketch(&mut sketch, &SolverOptions::default()).expect("solve fixture");
    (point_coordinates(&sketch), status, sketch.solve_state)
}

fn radius_values(sketch: &Sketch) -> Vec<(String, f64)> {
    sketch
        .entities
        .iter()
        .filter_map(|entity| {
            let (id, radius) = match entity {
                SketchEntity::Circle(circle) => (&circle.base.id, &circle.radius),
                SketchEntity::Arc(arc) => (&arc.base.id, &arc.radius),
                _ => return None,
            };
            let Coord::Literal(radius) = radius else {
                panic!("circular entity '{}' has a non-literal radius", id);
            };
            Some((id.as_str().to_string(), *radius))
        })
        .collect()
}

fn solve_radii_once(source: &Sketch) -> (Vec<(String, f64)>, SolveStatus, SolveState) {
    let mut sketch = source.clone();
    let status = solve_sketch(&mut sketch, &SolverOptions::default()).expect("solve fixture");
    (radius_values(&sketch), status, sketch.solve_state)
}

#[test]
fn serialized_fixture_validates_and_contains_supported_combination() {
    let path = fixture_path();
    let doc = validate_expanded_dir(&path).expect("fixture checksums and JSON");
    assert_eq!(doc.sketches.len(), 5);

    let mixed = sketch(&doc, "sketch:mixed");
    assert!(mixed
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, opencad_sketch::Constraint::Equal { .. })));
    assert!(mixed
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, opencad_sketch::Constraint::Parallel { .. })));
    assert!(mixed
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, opencad_sketch::Constraint::Perpendicular { .. })));

    let radii = sketch(&doc, "sketch:radius");
    assert!(radii.constraints.iter().any(|constraint| {
        matches!(
            constraint,
            opencad_sketch::Constraint::Equal {
                a: opencad_sketch::EqualTarget::Radius(_),
                b: opencad_sketch::EqualTarget::Radius(_),
                ..
            }
        )
    }));
}

#[test]
fn fixture_round_trip_is_byte_stable() {
    let source_path = fixture_path();
    let source = validate_expanded_dir(&source_path).expect("fixture");
    let temp = tempfile::tempdir().expect("tempdir");
    write_expanded_dir(temp.path(), &source).expect("write canonical fixture");

    for relative in [
        "manifest.ocad.json",
        "document.ocad.json",
        "graph/parameters.json",
        "graph/sketches.json",
        "graph/constraints.json",
        "graph/features.json",
        "graph/assemblies.json",
        "graph/materials.json",
        "graph/drawings.json",
        "graph/semantic_refs.json",
        "checksums.json",
    ] {
        let expected = fs::read(source_path.join(relative)).expect("source file");
        let actual = fs::read(temp.path().join(relative)).expect("round-trip file");
        assert_eq!(actual, expected, "canonical bytes changed for {relative}");
    }

    let restored = read_expanded_dir(temp.path()).expect("read round-trip");
    assert_eq!(source, restored);
}

#[test]
fn repeated_solves_have_identical_coordinates_dof_and_diagnostics() {
    let doc = validate_expanded_dir(fixture_path()).expect("fixture");

    let mixed_first = solve_once(sketch(&doc, "sketch:mixed"));
    let mixed_second = solve_once(sketch(&doc, "sketch:mixed"));
    assert_eq!(mixed_first, mixed_second);
    assert!(matches!(mixed_first.1, SolveStatus::Solved { .. }));
    assert!(matches!(mixed_first.2, SolveState::FullyConstrained));
    assert_near_point(&mixed_first.0, "ent:m_p1", [0.08, 0.0]);
    assert_near_point(&mixed_first.0, "ent:m_p3", [0.08, 0.0]);
    assert_near_point(&mixed_first.0, "ent:m_p5", [0.0, 0.04]);

    let radius_first = solve_radii_once(sketch(&doc, "sketch:radius"));
    let radius_second = solve_radii_once(sketch(&doc, "sketch:radius"));
    assert_eq!(radius_first, radius_second);
    assert!(matches!(radius_first.1, SolveStatus::Solved { .. }));
    assert!(matches!(radius_first.2, SolveState::FullyConstrained));
    let arc_radius = radius_first
        .0
        .iter()
        .find(|(id, _)| id == "ent:r_arc")
        .map(|(_, radius)| *radius)
        .expect("arc radius");
    assert!((arc_radius - 0.08).abs() < 1e-8);

    let under_first = solve_once(sketch(&doc, "sketch:under"));
    let under_second = solve_once(sketch(&doc, "sketch:under"));
    assert_eq!(under_first, under_second);
    assert!(matches!(
        under_first.1,
        SolveStatus::UnderConstrained { dof: 1, .. }
    ));
    assert!(matches!(
        under_first.2,
        SolveState::UnderConstrained { dof: 1 }
    ));

    let over_first = solve_once(sketch(&doc, "sketch:over"));
    let over_second = solve_once(sketch(&doc, "sketch:over"));
    assert_eq!(over_first, over_second);
    assert!(matches!(
        over_first.1,
        SolveStatus::OverConstrained { redundant: 1, .. }
    ));
    assert!(matches!(
        over_first.2,
        SolveState::OverConstrained { redundant: 1 }
    ));

    let contradictory_first = solve_once(sketch(&doc, "sketch:contradictory"));
    let contradictory_second = solve_once(sketch(&doc, "sketch:contradictory"));
    assert_eq!(contradictory_first, contradictory_second);
    let SolveStatus::Contradictory { message, .. } = &contradictory_first.1 else {
        panic!("expected contradictory status: {:?}", contradictory_first.1);
    };
    assert!(message.contains("constraints are contradictory"));
    let SolveState::Failed { message } = &contradictory_first.2 else {
        panic!("expected failed sketch state: {:?}", contradictory_first.2);
    };
    assert!(message.contains("contradiction threshold"));
    // A contradictory solve must not commit trial coordinates.
    assert_near_point(&contradictory_first.0, "ent:c_p1", [0.08, 0.0]);
}

#[test]
fn length_units_are_converted_to_internal_si_values() {
    assert!((parse_length_expr("80 mm").expect("mm") - 0.08).abs() < 1e-12);
    assert!((parse_length_expr("8 cm").expect("cm") - 0.08).abs() < 1e-12);
    assert!((parse_length_expr("0.08 m").expect("m") - 0.08).abs() < 1e-12);
    assert!((parse_length_expr("3.149606299 in").expect("in") - 0.08).abs() < 1e-9);
}

fn assert_near_point(points: &[(String, [f64; 2])], id: &str, expected: [f64; 2]) {
    let actual = points
        .iter()
        .find(|(point_id, _)| point_id == id)
        .map(|(_, point)| *point)
        .unwrap_or_else(|| panic!("missing point '{id}'"));
    assert!((actual[0] - expected[0]).abs() < 1e-8, "{id} x: {actual:?}");
    assert!((actual[1] - expected[1]).abs() < 1e-8, "{id} y: {actual:?}");
}
