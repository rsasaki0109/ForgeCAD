//! `opencad new` command (Task-121+).

use opencad_core::{DocumentId, DocumentMetadata, Result};
use opencad_feature::{bracket_hole_row, bracket_with_hole};
use opencad_file::{write_ocad, OcadDocument};
use opencad_graph::bracket_parameters;

/// Built-in sample document templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentTemplate {
    #[default]
    Bracket,
    HoleRow,
}

impl DocumentTemplate {
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "bracket" => Ok(Self::Bracket),
            "hole-row" => Ok(Self::HoleRow),
            _ => Err(opencad_core::OpenCadError::validation(format!(
                "unknown template '{name}'; expected 'bracket' or 'hole-row'"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bracket => "bracket",
            Self::HoleRow => "hole-row",
        }
    }
}

pub fn create_document(path: &str, template: DocumentTemplate) -> Result<()> {
    match template {
        DocumentTemplate::Bracket => create_bracket_document(path),
        DocumentTemplate::HoleRow => create_bracket_hole_row_document(path),
    }
}

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

pub fn create_bracket_hole_row_document(path: &str) -> Result<()> {
    let part = bracket_hole_row()?;
    let metadata = DocumentMetadata::new(
        DocumentId::new("doc:bracket_hole_row_001")?,
        "Bracket with Pin Hole Row",
    );
    let mut doc = OcadDocument::from_part_model(metadata, &part);
    doc.parameters = bracket_parameters();
    write_ocad(path, &doc)
}
