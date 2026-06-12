//! File checksum utilities.

use std::collections::BTreeMap;
use std::path::Path;

use opencad_core::{sha256_hex, OpenCadError, Result};
use serde::{Deserialize, Serialize};

/// Checksum manifest for files inside a `.ocad` container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecksumManifest {
    pub algorithm: String,
    pub files: BTreeMap<String, String>,
}

impl ChecksumManifest {
    pub fn compute(paths: &BTreeMap<String, Vec<u8>>) -> Self {
        let files = paths
            .iter()
            .map(|(path, bytes)| (path.clone(), sha256_hex(bytes)))
            .collect();
        Self {
            algorithm: "sha256".into(),
            files,
        }
    }

    pub fn verify(&self, paths: &BTreeMap<String, Vec<u8>>) -> Result<()> {
        for (path, expected) in &self.files {
            let actual_bytes = paths.get(path).ok_or_else(|| {
                OpenCadError::validation(format!("checksum entry missing file '{path}'"))
            })?;
            let actual = sha256_hex(actual_bytes);
            if &actual != expected {
                return Err(OpenCadError::ChecksumMismatch {
                    expected: expected.clone(),
                    actual,
                });
            }
        }
        Ok(())
    }
}

pub fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(io_error)?;
    Ok(sha256_hex(&bytes))
}

fn io_error(err: std::io::Error) -> OpenCadError {
    OpenCadError::Other(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_manifest_round_trip() {
        let mut files = BTreeMap::new();
        files.insert("graph/sketches.json".into(), br#"{"sketches":[]}"#.to_vec());
        let manifest = ChecksumManifest::compute(&files);
        manifest.verify(&files).expect("verify");
    }
}
