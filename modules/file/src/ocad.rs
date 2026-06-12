//! `.ocad` zip container format (Task-112+).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use opencad_core::{OpenCadError, Result};
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::document::OcadDocument;
use crate::expanded_dir::{is_expanded_dir, parse_document_files, serialize_document_files};

/// Write either `.ocad` zip or `.ocad.d` expanded directory based on path extension.
pub fn write_ocad(path: impl AsRef<Path>, doc: &OcadDocument) -> Result<()> {
    let path = path.as_ref();
    if path.extension().and_then(|s| s.to_str()) == Some("ocad") {
        write_ocad_zip(path, doc)
    } else {
        crate::expanded_dir::write_expanded_dir(path, doc)
    }
}

/// Write a `.ocad` zip archive.
pub fn write_ocad_zip(path: impl AsRef<Path>, doc: &OcadDocument) -> Result<()> {
    let path = path.as_ref();
    let files = serialize_document_files(doc)?;
    let file = File::create(path).map_err(io_error)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, bytes) in files {
        zip.start_file(name, options).map_err(zip_error)?;
        zip.write_all(&bytes).map_err(io_error)?;
    }

    zip.finish().map_err(zip_error)?;
    Ok(())
}

/// Read a `.ocad` zip archive.
pub fn read_ocad_zip(path: impl AsRef<Path>) -> Result<OcadDocument> {
    let path = path.as_ref();
    let file = File::open(path).map_err(io_error)?;
    let mut archive = ZipArchive::new(file).map_err(zip_error)?;
    let mut files = BTreeMap::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_error)?;
        let name = entry.name().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(io_error)?;
        files.insert(name, bytes);
    }

    parse_document_files(&files)
}

/// Read either `.ocad` zip or `.ocad.d` expanded directory.
pub fn read_ocad(path: impl AsRef<Path>) -> Result<OcadDocument> {
    let path = path.as_ref();
    if is_expanded_dir(path) {
        return crate::expanded_dir::read_expanded_dir(path);
    }
    if path.extension().and_then(|s| s.to_str()) == Some("ocad") {
        return read_ocad_zip(path);
    }
    if path.is_dir() {
        return crate::expanded_dir::read_expanded_dir(path);
    }
    Err(OpenCadError::validation(format!(
        "unsupported .ocad path '{}'",
        path.display()
    )))
}

/// Validate checksums and parse a `.ocad` zip or expanded directory.
pub fn validate_ocad(path: impl AsRef<Path>) -> Result<OcadDocument> {
    let path = path.as_ref();
    if is_expanded_dir(path) || path.is_dir() {
        return crate::expanded_dir::validate_expanded_dir(path);
    }
    read_ocad_zip(path)
}

fn io_error(err: std::io::Error) -> OpenCadError {
    OpenCadError::Other(err.to_string())
}

fn zip_error(err: zip::result::ZipError) -> OpenCadError {
    OpenCadError::Other(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencad_core::{DocumentId, DocumentMetadata};
    use opencad_feature::bracket_base_plate;

    fn bracket_document() -> OcadDocument {
        let part = bracket_base_plate().expect("model");
        let metadata = DocumentMetadata::new(
            DocumentId::new("doc:bracket_001").expect("id"),
            "Bracket Base Plate",
        );
        OcadDocument::from_part_model(metadata, &part)
    }

    #[test]
    fn zip_round_trip() {
        let doc = bracket_document();
        let dir = tempfile::tempdir().expect("tempdir");
        let zip_path = dir.path().join("bracket.ocad");
        write_ocad_zip(&zip_path, &doc).expect("write");
        let restored = read_ocad_zip(&zip_path).expect("read");
        assert_eq!(doc.sketches, restored.sketches);
        assert_eq!(doc.feature_nodes, restored.feature_nodes);
    }
}
