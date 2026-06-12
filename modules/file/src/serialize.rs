//! Canonical JSON serialization.

use serde::Serialize;

use opencad_core::{to_pretty_json, Result};

/// Serialize a value to deterministic, pretty-printed JSON with trailing newline.
pub fn to_canonical_json<T: Serialize>(value: &T) -> Result<String> {
    let mut json = to_pretty_json(value)?;
    json.push('\n');
    Ok(json)
}
