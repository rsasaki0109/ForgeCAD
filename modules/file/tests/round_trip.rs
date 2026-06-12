//! End-to-end `.ocad` round-trip with feature regeneration.

use opencad_core::{DocumentId, DocumentMetadata};
use opencad_feature::{bracket_base_plate, FeatureRegistry};
use opencad_file::{read_ocad_zip, write_expanded_dir, write_ocad_zip, OcadDocument};
use opencad_geometry::{GeometryKernel, MockGeometryKernel};
use opencad_graph::bracket_parameters;

#[test]
fn bracket_round_trip_regenerates_after_read() {
    let part = bracket_base_plate().expect("model");
    let metadata = DocumentMetadata::new(
        DocumentId::new("doc:bracket_001").expect("id"),
        "Bracket Base Plate",
    );
    let mut doc = OcadDocument::from_part_model(metadata, &part);
    doc.parameters = bracket_parameters();

    let dir = tempfile::tempdir().expect("tempdir");
    write_expanded_dir(dir.path(), &doc).expect("write dir");
    let zip_path = dir.path().join("bracket.ocad");
    write_ocad_zip(&zip_path, &doc).expect("write zip");

    for restored in [
        opencad_file::read_expanded_dir(dir.path()).expect("read dir"),
        read_ocad_zip(&zip_path).expect("read zip"),
    ] {
        let params = restored.parameters.clone();
        let mut model = restored.into_part_model();
        let kernel = MockGeometryKernel::new();
        let registry = FeatureRegistry::with_defaults();
        let report = model
            .regenerate(&kernel, &registry, Some(&params), None)
            .expect("regen");
        assert_eq!(report.regenerated.len(), 2);
        let body = model.active_body().expect("body");
        let mass = kernel.mass_properties(body, 2700.0).expect("mass");
        assert!(mass.volume_m3 > 0.0);
    }
}
