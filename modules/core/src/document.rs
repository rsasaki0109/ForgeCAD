use serde::{Deserialize, Serialize};

use crate::id::DocumentId;
use crate::units::LengthUnit;

/// Document-level metadata stored in `.ocad`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub id: DocumentId,
    pub name: String,
    pub units: LengthUnit,
    pub created_with: String,
    pub schema_version: String,
}

impl DocumentMetadata {
    pub fn new(id: DocumentId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            units: LengthUnit::Millimeter,
            created_with: format!("OpenCAD {}", env!("CARGO_PKG_VERSION")),
            schema_version: "opencad.document.v0.1".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_round_trip() {
        let meta =
            DocumentMetadata::new(DocumentId::new("doc:test").expect("valid id"), "Test Part");
        let json = serde_json::to_string(&meta).expect("serialize");
        let restored: DocumentMetadata = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(meta, restored);
    }
}
