//! Manifest read/write for `.ocad` containers.

use opencad_core::{OcadManifest, Result};

use crate::document::OcadDocument;
use crate::serialize::to_canonical_json;

pub const MANIFEST_FILE: &str = "manifest.ocad.json";

pub fn manifest_for_document(doc: &OcadDocument) -> OcadManifest {
    let mut manifest = OcadManifest::new_v0_1(doc.metadata.id.as_str());
    manifest.created_at = "1970-01-01T00:00:00Z".into();
    manifest
}

pub fn manifest_json(doc: &OcadDocument) -> Result<String> {
    to_canonical_json(&manifest_for_document(doc))
}
