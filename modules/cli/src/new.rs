//! `opencad new` command (Task-121+).

use opencad_core::{DocumentId, DocumentMetadata, Result};
use opencad_feature::bracket_with_hole;
use opencad_file::{write_ocad, OcadDocument};
use opencad_graph::bracket_parameters;

pub fn create_bracket_document(path: &str) -> Result<()> {
    let part = bracket_with_hole()?;
    let metadata = DocumentMetadata::new(
        DocumentId::new("doc:bracket_001")?,
        "Bracket with Mounting Hole",
    );
    let mut doc = OcadDocument::from_part_model(metadata, &part);
    doc.parameters = bracket_parameters();
    write_ocad(path, &doc)
}
