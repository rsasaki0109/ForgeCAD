//! Reusable headless acceptance flow for the desktop distribution.
//!
//! The flow intentionally works on a copy of an expanded document.  It is
//! suitable for both the workspace integration test and the packaged Tauri
//! binary, and it never writes generated geometry back to the source fixture.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use opencad_core::{DocumentKind, OpenCadError, Result};
use opencad_file::validate_ocad;
use serde::{Deserialize, Serialize};

use crate::export::export_stl_document;
use crate::inspect::inspect_document;
use crate::parameters::{list_document_parameters, set_document_parameter};
use crate::pick::{pick_document, PickOptions, PickTarget};
use crate::preview::preview_document;
use crate::regenerate::regenerate_document;

/// Stable, serializable evidence returned by [`run_desktop_smoke`].
///
/// Counts and identifiers are deliberately used instead of returning a PNG,
/// mesh, or kernel handle.  The Design Graph remains the source of truth and
/// all generated geometry stays disposable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopSmokeSummary {
    pub source: String,
    pub work_dir: String,
    pub document_name: String,
    pub document_kind: DocumentKind,
    pub copied_files: usize,
    pub source_unchanged: bool,
    pub preview_triangles: usize,
    pub preview_vertices: usize,
    pub edited_parameter_id: String,
    pub width_before_mm: f64,
    pub width_after_mm: f64,
    pub regenerated_features: usize,
    pub regeneration_triangles: usize,
    pub picked_target: PickTarget,
    pub pick_highlight_segments: usize,
    pub exported_triangles: usize,
    pub export_path: String,
}

/// Run the desktop open/preview/edit/regenerate/pick/export smoke contract.
///
/// `source` must be an expanded `.ocad.d` directory containing a part
/// document.  `work_dir` is created by this function and must not already
/// exist; refusing existing paths prevents an acceptance run from silently
/// overwriting a user's document or a previous run's evidence.
pub fn run_desktop_smoke(source: &Path, work_dir: &Path) -> Result<DesktopSmokeSummary> {
    if !source.is_dir() {
        return Err(OpenCadError::validation(format!(
            "desktop smoke source must be an expanded document directory: {}",
            source.display()
        )));
    }

    let source_files = snapshot_files(source)?;
    let source_doc = validate_ocad(source)?;
    ensure_part_document(source_doc.metadata.kind, "desktop smoke")?;

    create_new_directory(work_dir)?;
    copy_directory_contents(source, work_dir)?;

    let opened = validate_ocad(work_dir)?;
    ensure_part_document(opened.metadata.kind, "desktop smoke copy")?;
    let inspect = inspect_document(path_string(work_dir).as_str())?;
    if inspect.name.is_empty() || inspect.features == 0 {
        return Err(OpenCadError::validation(
            "desktop smoke document must contain a named feature model",
        ));
    }

    let work_path = path_string(work_dir);
    let preview = preview_document(&work_path)?;
    if preview.triangles == 0 || preview.vertices == 0 || preview.png_base64.is_empty() {
        return Err(OpenCadError::validation(
            "desktop smoke preview did not produce geometry and an image",
        ));
    }
    let preview_png = STANDARD
        .decode(&preview.png_base64)
        .map_err(|err| OpenCadError::validation(format!("invalid preview base64: {err}")))?;
    image::load_from_memory(&preview_png)
        .map_err(|err| OpenCadError::validation(format!("invalid preview image: {err}")))?;

    let width_before = list_document_parameters(&work_path)?
        .into_iter()
        .find(|row| row.id == "param:width")
        .ok_or_else(|| OpenCadError::validation("desktop smoke requires param:width"))?;
    let width_before_mm = width_before
        .value_mm
        .ok_or_else(|| OpenCadError::validation("desktop smoke param:width has no length value"))?;
    if width_before.expr != "80 mm" {
        return Err(OpenCadError::validation(format!(
            "desktop smoke expected param:width to start at 80 mm, got '{}'",
            width_before.expr
        )));
    }

    set_document_parameter(&work_path, "param:width", "100 mm")?;
    let width_after = list_document_parameters(&work_path)?
        .into_iter()
        .find(|row| row.id == "param:width")
        .ok_or_else(|| OpenCadError::validation("edited param:width disappeared"))?;
    let width_after_mm = width_after
        .value_mm
        .ok_or_else(|| OpenCadError::validation("edited param:width has no length value"))?;
    if (width_after_mm - 100.0).abs() > 1e-9 {
        return Err(OpenCadError::validation(format!(
            "desktop smoke parameter edit did not reach 100 mm: {width_after_mm}"
        )));
    }

    let regeneration = regenerate_document(&work_path)?;
    if regeneration.regenerated.is_empty() || regeneration.triangles == 0 {
        return Err(OpenCadError::validation(
            "desktop smoke regeneration did not produce feature and mesh evidence",
        ));
    }

    let picked = pick_document(&work_path, &PickOptions::default())?;
    if picked.triangle_count == 0
        || matches!(picked.selection, PickTarget::None)
        || picked.highlight_segments_px.is_empty()
    {
        return Err(OpenCadError::validation(
            "desktop smoke pick did not select geometry with highlight evidence",
        ));
    }

    let export_path = work_dir.join("desktop-smoke.stl");
    let export = export_stl_document(&work_path, &path_string(&export_path))?;
    let export_size = fs::metadata(&export_path)
        .map_err(|err| io_error(&export_path, err))?
        .len();
    if export.triangles == 0 || !export_path.is_file() || export_size <= 84 {
        return Err(OpenCadError::validation(
            "desktop smoke export did not produce a non-empty binary STL",
        ));
    }

    validate_ocad(work_dir)?;
    let source_unchanged = source_files == snapshot_files(source)?;
    if !source_unchanged {
        return Err(OpenCadError::validation(
            "desktop smoke modified the source fixture",
        ));
    }

    Ok(DesktopSmokeSummary {
        source: path_string(source),
        work_dir: work_path,
        document_name: inspect.name,
        document_kind: opened.metadata.kind,
        copied_files: source_files.len(),
        source_unchanged,
        preview_triangles: preview.triangles,
        preview_vertices: preview.vertices,
        edited_parameter_id: "param:width".into(),
        width_before_mm,
        width_after_mm,
        regenerated_features: regeneration.regenerated.len(),
        regeneration_triangles: regeneration.triangles,
        picked_target: picked.selection,
        pick_highlight_segments: picked.highlight_segments_px.len(),
        exported_triangles: export.triangles,
        export_path: export.output,
    })
}

fn ensure_part_document(kind: DocumentKind, operation: &str) -> Result<()> {
    if kind != DocumentKind::Part {
        return Err(OpenCadError::validation(format!(
            "{operation} supports only DocumentKind::Part, got {kind:?}"
        )));
    }
    Ok(())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn create_new_directory(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(OpenCadError::validation(format!(
            "desktop smoke work directory already exists: {}",
            path.display()
        )));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| io_error(parent, err))?;
        }
    }
    fs::create_dir(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::AlreadyExists {
            OpenCadError::validation(format!(
                "desktop smoke work directory already exists: {}",
                path.display()
            ))
        } else {
            io_error(path, err)
        }
    })
}

fn copy_directory_contents(source: &Path, destination: &Path) -> Result<()> {
    for entry in sorted_entries(source)? {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|err| io_error(&source_path, err))?;
        if file_type.is_symlink() {
            return Err(OpenCadError::validation(format!(
                "desktop smoke source cannot contain symlink: {}",
                source_path.display()
            )));
        }
        if file_type.is_dir() {
            fs::create_dir(&destination_path).map_err(|err| io_error(&destination_path, err))?;
            copy_directory_contents(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path)
                .map_err(|err| io_error(&destination_path, err))?;
        } else {
            return Err(OpenCadError::validation(format!(
                "desktop smoke source contains unsupported entry: {}",
                source_path.display()
            )));
        }
    }
    Ok(())
}

fn snapshot_files(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    snapshot_files_inner(root, root, &mut files)?;
    Ok(files)
}

fn snapshot_files_inner(
    root: &Path,
    current: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    for entry in sorted_entries(current)? {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|err| io_error(&path, err))?;
        if file_type.is_symlink() {
            return Err(OpenCadError::validation(format!(
                "desktop smoke source cannot contain symlink: {}",
                path.display()
            )));
        }
        if file_type.is_dir() {
            snapshot_files_inner(root, &path, files)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| OpenCadError::validation("invalid smoke fixture path"))?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path).map_err(|err| io_error(&path, err))?;
            files.insert(relative, bytes);
        }
    }
    Ok(())
}

fn sorted_entries(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)
        .map_err(|err| io_error(path, err))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|err| io_error(path, err))?;
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

fn io_error(path: &Path, err: std::io::Error) -> OpenCadError {
    OpenCadError::Other(format!("{}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn existing_work_directory_is_rejected_before_copy() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let work = temp.path().join("work");
        fs::create_dir(&source).expect("source");
        fs::create_dir(&work).expect("work");

        let err = create_new_directory(&work).expect_err("existing workdir must be rejected");
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn file_snapshot_is_ordered_and_byte_exact() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("document.ocad.d");
        fs::create_dir_all(root.join("graph")).expect("graph");
        fs::write(root.join("graph/b.json"), b"b").expect("b");
        fs::write(root.join("graph/a.json"), b"a").expect("a");

        let first = snapshot_files(&root).expect("snapshot");
        fs::write(root.join("graph/a.json"), b"changed").expect("change");
        let second = snapshot_files(&root).expect("snapshot");
        assert_ne!(first, second);
        assert_eq!(
            first.keys().collect::<Vec<_>>(),
            second.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn only_part_documents_are_accepted() {
        for kind in [DocumentKind::Assembly, DocumentKind::Drawing] {
            let error = ensure_part_document(kind, "smoke").expect_err("non-part kind");
            assert!(error.to_string().contains("only DocumentKind::Part"));
        }
        ensure_part_document(DocumentKind::Part, "smoke").expect("part kind");
    }
}
