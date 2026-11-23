//! Persistent vault sync observations.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::vault::VaultRuntimeError;

pub(crate) type Bytes = Option<Vec<u8>>;
pub(crate) type Snapshot = BTreeMap<String, Bytes>;
pub(crate) type Observed = BTreeMap<String, (Bytes, Bytes)>;

pub(crate) fn load(root: &Path) -> Result<Observed, VaultRuntimeError> {
    let path = manifest_path(root);
    let Ok(bytes) = fs::read(path) else {
        return Ok(BTreeMap::new());
    };
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| VaultRuntimeError::Unavailable)?;
    let Value::Object(entries) = value else {
        return Err(VaultRuntimeError::Unavailable);
    };
    entries.into_iter().map(parse_observation).collect()
}

pub(crate) fn save(
    root: &Path,
    private: &Snapshot,
    shared: &Snapshot,
) -> Result<(), VaultRuntimeError> {
    let paths = private
        .keys()
        .chain(shared.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let entries = paths
        .into_iter()
        .map(|path| {
            let mut fields = Map::new();
            fields.insert("private".into(), encode(private.get(&path)));
            fields.insert("shared".into(), encode(shared.get(&path)));
            (path, Value::Object(fields))
        })
        .collect::<Map<_, _>>();
    let folder = root.join(".tether-sync");
    fs::create_dir_all(&folder).map_err(|_| VaultRuntimeError::Unavailable)?;
    let bytes =
        serde_json::to_vec(&Value::Object(entries)).map_err(|_| VaultRuntimeError::Unavailable)?;
    fs::write(manifest_path(root), bytes).map_err(|_| VaultRuntimeError::Unavailable)
}

fn parse_observation(
    (path, value): (String, Value),
) -> Result<(String, (Bytes, Bytes)), VaultRuntimeError> {
    let Value::Object(mut fields) = value else {
        return Err(VaultRuntimeError::Unavailable);
    };
    let private = decode(fields.remove("private"))?;
    let shared = decode(fields.remove("shared"))?;
    Ok((path, (private, shared)))
}

fn encode(value: Option<&Bytes>) -> Value {
    value
        .and_then(Clone::clone)
        .map_or(Value::Null, |bytes| Value::String(encode_hex(&bytes)))
}

fn decode(value: Option<Value>) -> Result<Bytes, VaultRuntimeError> {
    match value {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(text)) => decode_hex(&text).map(Some),
        _ => Err(VaultRuntimeError::Unavailable),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        if write!(encoded, "{byte:02x}").is_err() {
            return String::new();
        }
    }
    encoded
}

fn decode_hex(text: &str) -> Result<Vec<u8>, VaultRuntimeError> {
    if !text.len().is_multiple_of(2) {
        return Err(VaultRuntimeError::Unavailable);
    }
    (0..text.len())
        .step_by(2)
        .map(|index| {
            let high = hex_digit(text.as_bytes()[index])?;
            let low = hex_digit(text.as_bytes()[index + 1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_digit(byte: u8) -> Result<u8, VaultRuntimeError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(VaultRuntimeError::Unavailable),
    }
}

fn manifest_path(root: &Path) -> PathBuf {
    root.join(".tether-sync").join("manifest.json")
}
