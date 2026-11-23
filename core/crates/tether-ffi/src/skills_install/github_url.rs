//! Parses GitHub skill URLs into the owner, repo, ref, and folder parts.

use reqwest::Url;

use crate::skills::SkillsRuntimeError;

/// Builds the codeload ZIP URL for a validated GitHub skill URL.
pub(super) fn github_zip_url(url: &str) -> Result<String, SkillsRuntimeError> {
    let (_, _, reference, _) = parse_github_url(url)?;
    let parsed = Url::parse(url).map_err(|_| SkillsRuntimeError::InvalidUrl)?;
    let segments = parsed
        .path_segments()
        .ok_or(SkillsRuntimeError::InvalidUrl)?
        .collect::<Vec<_>>();
    Ok(format!(
        "https://codeload.github.com/{}/{}/zip/{}",
        segments[0],
        segments[1],
        reference.as_deref().unwrap_or("HEAD")
    ))
}

/// Returns owner, repo, optional ref, and optional folder path.
pub(super) fn parse_github_url(
    url: &str,
) -> Result<(String, String, Option<String>, Option<String>), SkillsRuntimeError> {
    let parsed = Url::parse(url).map_err(|_| SkillsRuntimeError::InvalidUrl)?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(SkillsRuntimeError::InvalidUrl);
    }
    let segments = parsed
        .path_segments()
        .ok_or(SkillsRuntimeError::InvalidUrl)?
        .collect::<Vec<_>>();
    if segments.len() == 2 {
        return Ok((segments[0].into(), segments[1].into(), None, None));
    }
    if segments.len() >= 5 && segments[2] == "tree" {
        return Ok((
            segments[0].into(),
            segments[1].into(),
            Some(segments[3].into()),
            Some(segments[4..].join("/")),
        ));
    }
    Err(SkillsRuntimeError::InvalidUrl)
}
