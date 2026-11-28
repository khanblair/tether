//! Parse and validate open Agent Skills documents.

use std::collections::BTreeMap;

mod discovery;
mod frontmatter;
mod index;
pub mod samples;
mod scan;
mod scan_rules;
mod upkeep;

use frontmatter::{Frontmatter, frontmatter_fields};

pub use discovery::{
    BundledSkill, DiscoverInput, Skill, SkillIssue, SkillSource, SkillView, SkillsCatalog,
};
pub use index::skills_index;
pub use samples::{
    SampleFile, SampleInstall, SampleSkill, install_sample, sample_notice, sample_skills,
};
pub use scan::{ScanFinding, ScanReport, ScanSeverity, ScanVerdict, SkillFile, scan_skill};
pub use upkeep::{SkillScope, SkillState, SkillUsage, UpkeepAction, plan_upkeep};

/// A validated `SKILL.md` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDocument {
    /// Directory name and public skill name.
    pub name: String,
    /// Short trigger description.
    pub description: String,
    /// Optional license text.
    pub license: Option<String>,
    /// Optional runtime compatibility note.
    pub compatibility: Option<String>,
    /// Optional allowed tool names.
    pub allowed_tools: Option<String>,
    /// Extra string fields for consumers.
    pub metadata: BTreeMap<String, String>,
    /// Markdown body below the frontmatter.
    pub body: String,
}

/// A `SKILL.md` validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillError {
    /// The YAML frontmatter delimiters are missing.
    MissingFrontmatter,
    /// A required field is missing or blank.
    MissingField(&'static str),
    /// The skill name is invalid or differs from its folder.
    InvalidName,
    /// The description is longer than the open format allows.
    DescriptionTooLong,
    /// A line cannot be read as a scalar frontmatter field.
    InvalidFrontmatter,
    /// The requested skill file path is outside an allowed folder.
    InvalidPath,
    /// A skill file is a symbolic link.
    Symlink,
    /// A skill file is larger than the supported limit.
    FileTooLarge,
    /// A skill file is not valid UTF-8.
    InvalidUtf8,
    /// A skill file could not be read.
    Io(String),
    /// The requested skill or file does not exist.
    NotFound,
}

/// Parses the common scalar form of Agent Skills frontmatter.
///
/// # Errors
/// Returns a validation error for malformed fields or a name that differs from `folder`.
pub fn parse_skill(folder: &str, source: &str) -> Result<SkillDocument, SkillError> {
    let Frontmatter {
        mut fields,
        metadata,
        body,
    } = frontmatter_fields(source)?;
    let name = fields
        .remove("name")
        .ok_or(SkillError::MissingField("name"))?;
    let valid = !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !valid || name != folder {
        return Err(SkillError::InvalidName);
    }
    let description = fields
        .remove("description")
        .ok_or(SkillError::MissingField("description"))?;
    let description = description
        .strip_suffix('\n')
        .unwrap_or(&description)
        .to_owned();
    if description.trim().is_empty() {
        return Err(SkillError::MissingField("description"));
    }
    if description.chars().count() > 1024 {
        return Err(SkillError::DescriptionTooLong);
    }
    Ok(SkillDocument {
        name,
        description,
        license: fields.remove("license"),
        compatibility: fields.remove("compatibility"),
        allowed_tools: fields.remove("allowed-tools"),
        metadata,
        body,
    })
}

#[cfg(test)]
mod frontmatter_tests;
#[cfg(test)]
mod scan_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod upkeep_tests;
