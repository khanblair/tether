//! Parsing of GitHub's SHA256SUMS asset.

use super::UpdateError;

pub(crate) fn digest_for(bytes: &[u8], apk_name: &str) -> Result<[u8; 32], UpdateError> {
    let text = std::str::from_utf8(bytes).map_err(|_| UpdateError::Invalid)?;
    let mut found = None;
    for line in text.lines() {
        let Some((digest, name)) = line.split_once("  ") else {
            continue;
        };
        let matches = name == apk_name || name.strip_prefix('*') == Some(apk_name);
        if !matches {
            continue;
        }
        if found.is_some() {
            return Err(UpdateError::Invalid);
        }
        found = Some(parse_digest(digest)?);
    }
    found.ok_or(UpdateError::Invalid)
}

fn parse_digest(value: &str) -> Result<[u8; 32], UpdateError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(UpdateError::Invalid);
    }
    let mut digest = [0_u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        digest[index] = (hex(pair[0])? << 4) | hex(pair[1])?;
    }
    Ok(digest)
}

fn hex(byte: u8) -> Result<u8, UpdateError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(UpdateError::Invalid),
    }
}
