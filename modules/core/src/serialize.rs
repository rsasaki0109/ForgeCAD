use indexmap::IndexMap;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::Result;

/// Serialize a value to deterministic, pretty-printed JSON.
///
/// Keys in `IndexMap` fields preserve insertion order; callers must insert
/// keys in a stable order for fully deterministic output.
pub fn to_pretty_json<T: Serialize>(value: &T) -> Result<String> {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"  ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value.serialize(&mut serializer)?;
    Ok(String::from_utf8(buf).expect("JSON must be valid UTF-8"))
}

/// Serialize a map with sorted keys for deterministic output.
pub fn sorted_map<K, V>(map: &IndexMap<K, V>) -> IndexMap<K, V>
where
    K: Ord + Clone + std::hash::Hash,
    V: Clone,
{
    let mut keys: Vec<K> = map.keys().cloned().collect();
    keys.sort();
    let mut sorted = IndexMap::new();
    for key in keys {
        if let Some(value) = map.get(&key) {
            sorted.insert(key, value.clone());
        }
    }
    sorted
}

/// Compute a SHA-256 hex digest of the input bytes.
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Sample {
        b: u32,
        a: u32,
    }

    #[test]
    fn pretty_json_is_stable() {
        let sample = Sample { a: 1, b: 2 };
        let first = to_pretty_json(&sample).expect("serialize");
        let second = to_pretty_json(&sample).expect("serialize");
        assert_eq!(first, second);
        assert!(first.contains("\"a\": 1"));
    }

    #[test]
    fn sha256_is_deterministic() {
        let a = sha256_hex(b"opencad");
        let b = sha256_hex(b"opencad");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }
}
