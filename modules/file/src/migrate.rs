//! Schema version migrations (Task-116+).

use opencad_core::Result;

use crate::document::OcadDocument;

/// No-op migration hook for MVP (`0.1.0` only).
pub fn migrate_to_current(doc: OcadDocument) -> Result<OcadDocument> {
    Ok(doc)
}
