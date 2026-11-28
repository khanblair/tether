//! UniFFI access to the sample skills bundled with the application.

use tether_skills::{SampleInstall, SkillError, parse_skill, sample_notice};

use crate::skills::{SkillsRuntime, SkillsRuntimeError};

/// A bundled sample skill and its install state.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SampleSkillView {
    /// Sample skill name.
    pub name: String,
    /// Trigger description from the sample frontmatter.
    pub description: String,
    /// Number of embedded files.
    pub file_count: u32,
    /// Whether a user skill with this name is installed.
    pub installed: bool,
}

#[uniffi::export]
impl SkillsRuntime {
    /// Lists the bundled sample skills and their install state.
    ///
    /// # Errors
    /// Returns InvalidSkill if an embedded sample cannot be parsed.
    pub fn sample_skills(&self) -> Result<Vec<SampleSkillView>, SkillsRuntimeError> {
        tether_skills::sample_skills()
            .iter()
            .map(|sample| sample_view(self, sample))
            .collect()
    }

    /// Installs one bundled sample skill into the user skills folder.
    ///
    /// # Errors
    /// Returns NotFound for an unknown sample or a mapped skills storage error.
    #[expect(clippy::needless_pass_by_value, reason = "UniFFI passes owned values")]
    pub fn install_sample_skill(&self, name: String) -> Result<(), SkillsRuntimeError> {
        match tether_skills::install_sample(&self.root, &name)
            .map_err(|error| map_skill_error(&error))?
        {
            SampleInstall::Installed | SampleInstall::AlreadyInstalled => Ok(()),
        }
    }

    /// Returns the bundled sample attribution notice and license texts.
    #[must_use]
    pub fn sample_skills_notice(&self) -> String {
        sample_notice()
    }
}

fn sample_view(
    runtime: &SkillsRuntime,
    sample: &tether_skills::SampleSkill,
) -> Result<SampleSkillView, SkillsRuntimeError> {
    let source = sample
        .files
        .iter()
        .find(|file| file.path == "SKILL.md")
        .map(|file| file.contents)
        .ok_or(SkillsRuntimeError::InvalidSkill)?;
    let document = parse_skill(sample.name, source).map_err(|error| map_skill_error(&error))?;
    let file_count =
        u32::try_from(sample.files.len()).map_err(|_| SkillsRuntimeError::Unavailable)?;
    Ok(SampleSkillView {
        name: sample.name.to_owned(),
        description: document.description,
        file_count,
        installed: runtime.root.join(sample.name).is_dir(),
    })
}

fn map_skill_error(error: &SkillError) -> SkillsRuntimeError {
    match error {
        SkillError::NotFound => SkillsRuntimeError::NotFound,
        SkillError::InvalidPath | SkillError::Symlink => SkillsRuntimeError::InvalidPath,
        SkillError::Io(_) => SkillsRuntimeError::Unavailable,
        SkillError::MissingFrontmatter
        | SkillError::MissingField(_)
        | SkillError::InvalidName
        | SkillError::DescriptionTooLong
        | SkillError::InvalidFrontmatter
        | SkillError::FileTooLarge
        | SkillError::InvalidUtf8 => SkillsRuntimeError::InvalidSkill,
    }
}

#[cfg(test)]
#[path = "skills_samples_tests.rs"]
mod skills_samples_tests;
