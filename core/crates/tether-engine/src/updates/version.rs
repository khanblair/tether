//! Strict three-part version parsing for release comparison.

use super::UpdateError;

/// A numeric three-part application version.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Version {
    /// Major component.
    pub(crate) major: u64,
    /// Minor component.
    pub(crate) minor: u64,
    /// Patch component.
    pub(crate) patch: u64,
}

impl Version {
    pub(crate) fn as_bare(self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub(crate) fn parse_current(value: &str) -> Result<Version, UpdateError> {
    parse_parts(value).ok_or(UpdateError::Invalid)
}

pub(crate) fn parse_tag(value: &str) -> Result<Version, UpdateError> {
    let Some(bare) = value.strip_prefix('v') else {
        return Err(UpdateError::Invalid);
    };
    parse_parts(bare).ok_or(UpdateError::Invalid)
}

fn parse_parts(value: &str) -> Option<Version> {
    let mut parts = value.split('.');
    let version = Version {
        major: parse_number(parts.next()?)?,
        minor: parse_number(parts.next()?)?,
        patch: parse_number(parts.next()?)?,
    };
    parts.next().is_none().then_some(version)
}

fn parse_number(value: &str) -> Option<u64> {
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}
