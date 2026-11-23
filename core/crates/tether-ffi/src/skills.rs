//! Read-only skill discovery for the platform.

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path};
use std::sync::Arc;

use tether_skills::{SkillDocument, SkillError, parse_skill};

use crate::memory::checked_directory;

/// A skill row, including a validation failure when present.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SkillView {
    /// Skill folder name.
    pub folder: String,
    /// Parsed skill name, or empty for an invalid skill.
    pub name: String,
    /// Parsed description, or empty for an invalid skill.
    pub description: String,
    /// Whether the file parsed successfully.
    pub valid: bool,
    /// Validation message for an invalid skill.
    pub error: Option<String>,
}

/// The parsed skill fields and Markdown body.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SkillDetailView {
    /// Skill folder name.
    pub folder: String,
    /// Parsed public name.
    pub name: String,
    /// Trigger description.
    pub description: String,
    /// Optional license.
    pub license: Option<String>,
    /// Optional compatibility note.
    pub compatibility: Option<String>,
    /// Optional allowed tool names.
    pub allowed_tools: Option<String>,
    /// Extra scalar metadata.
    pub metadata: HashMap<String, String>,
    /// Markdown body.
    pub body: String,
}

/// A skill read failed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SkillsRuntimeError {
    /// The directory or folder name is unsafe.
    #[error("invalid skill path")]
    InvalidPath,
    /// The requested skill folder is absent.
    #[error("skill not found")]
    NotFound,
    /// The skill file did not pass validation.
    #[error("invalid skill")]
    InvalidSkill,
    /// Storage could not be read.
    #[error("skills are unavailable")]
    Unavailable,
    /// The install source URL is not an allowed GitHub URL.
    #[error("invalid skill URL")]
    InvalidUrl,
    /// The install source could not be downloaded.
    #[error("skill download failed")]
    Download,
    /// The install archive exceeds a configured limit.
    #[error("skill archive is too large")]
    TooLarge,
    /// The source does not contain one valid skill.
    #[error("source is not a skill")]
    NotASkill,
    /// The source contains more than one skill.
    #[error("source contains several skills")]
    SeveralSkills {
        /// Skill names found in the source.
        names: Vec<String>,
    },
    /// The static scan blocked installation.
    #[error("skill scan blocked installation")]
    Blocked,
    /// The project is not trusted for project-scoped installation.
    #[error("project is not trusted")]
    NotTrusted,
    /// A skill with the same name is already installed.
    #[error("skill is already installed")]
    AlreadyInstalled,
}

/// Reads `SKILL.md` files beneath one skill directory.
#[derive(uniffi::Object)]
pub struct SkillsRuntime {
    pub(crate) root: std::path::PathBuf,
    pub(crate) clock: Arc<dyn Fn() -> i64 + Send + Sync>,
}

#[uniffi::export]
impl SkillsRuntime {
    /// Opens the skill directory, creating it when missing.
    ///
    /// # Errors
    /// Returns an invalid directory or an I/O failure.
    #[uniffi::constructor]
    pub fn new(skills_dir: &str) -> Result<Arc<Self>, SkillsRuntimeError> {
        let root = checked_directory(skills_dir).ok_or(SkillsRuntimeError::InvalidPath)?;
        fs::create_dir_all(root).map_err(|_| SkillsRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            root: root.to_path_buf(),
            clock: Arc::new(crate::skills_install::system_now_ms),
        }))
    }

    /// Lists direct child skill folders in name order.
    ///
    /// # Errors
    /// Returns `Unavailable` when the directory cannot be listed or a file cannot be read.
    pub fn list(&self) -> Result<Vec<SkillView>, SkillsRuntimeError> {
        skill_folders(&self.root)?
            .into_iter()
            .map(|folder| self.list_one(folder))
            .collect()
    }

    /// Loads one parsed skill and its Markdown body.
    ///
    /// # Errors
    /// Returns invalid path, missing skill, invalid file, or storage errors.
    pub fn read(&self, folder: String) -> Result<SkillDetailView, SkillsRuntimeError> {
        if !valid_folder(&folder) {
            return Err(SkillsRuntimeError::InvalidPath);
        }
        let path = self.root.join(&folder);
        if fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(SkillsRuntimeError::InvalidPath);
        }
        let file = path.join("SKILL.md");
        if is_symlink(&file) {
            return Err(SkillsRuntimeError::InvalidPath);
        }
        let source = fs::read_to_string(file).map_err(|error| read_error(&error))?;
        let document =
            parse_skill(&folder, &source).map_err(|_| SkillsRuntimeError::InvalidSkill)?;
        Ok(detail(folder, document))
    }
}

impl SkillsRuntime {
    fn list_one(&self, folder: String) -> Result<SkillView, SkillsRuntimeError> {
        let file = self.root.join(&folder).join("SKILL.md");
        if is_symlink(&file) {
            return Ok(invalid(folder, "symlinked SKILL.md"));
        }
        let source = fs::read_to_string(file);
        match source {
            Ok(source) => match parse_skill(&folder, &source) {
                Ok(skill) => Ok(SkillView {
                    folder,
                    name: skill.name,
                    description: skill.description,
                    valid: true,
                    error: None,
                }),
                Err(error) => Ok(invalid(folder, skill_error(&error))),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(invalid(folder, "SKILL.md not found"))
            }
            Err(_) => Err(SkillsRuntimeError::Unavailable),
        }
    }
}

fn detail(folder: String, document: SkillDocument) -> SkillDetailView {
    SkillDetailView {
        folder,
        name: document.name,
        description: document.description,
        license: document.license,
        compatibility: document.compatibility,
        allowed_tools: document.allowed_tools,
        metadata: document.metadata.into_iter().collect(),
        body: document.body,
    }
}

fn invalid(folder: String, error: &str) -> SkillView {
    SkillView {
        folder,
        name: String::new(),
        description: String::new(),
        valid: false,
        error: Some(error.to_owned()),
    }
}

fn skill_error(error: &SkillError) -> &'static str {
    match error {
        SkillError::MissingFrontmatter => "missing frontmatter",
        SkillError::MissingField(_) => "missing required field",
        SkillError::InvalidName => "invalid skill name",
        SkillError::DescriptionTooLong => "description is too long",
        SkillError::InvalidFrontmatter => "invalid frontmatter",
        SkillError::InvalidPath => "invalid skill path",
        SkillError::Symlink => "symlinked skill path",
        SkillError::FileTooLarge => "skill file is too large",
        SkillError::InvalidUtf8 => "skill file is not UTF-8",
        SkillError::Io(_) => "skill file could not be read",
        SkillError::NotFound => "skill file was not found",
    }
}

fn read_error(error: &std::io::Error) -> SkillsRuntimeError {
    if error.kind() == std::io::ErrorKind::NotFound {
        SkillsRuntimeError::NotFound
    } else {
        SkillsRuntimeError::Unavailable
    }
}

fn valid_folder(folder: &str) -> bool {
    !folder.starts_with('.')
        && matches!(
            Path::new(folder)
                .components()
                .collect::<Vec<_>>()
                .as_slice(),
            [Component::Normal(_)]
        )
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

fn skill_folders(root: &Path) -> Result<Vec<String>, SkillsRuntimeError> {
    let mut folders = Vec::new();
    for entry in fs::read_dir(root).map_err(|_| SkillsRuntimeError::Unavailable)? {
        let entry = entry.map_err(|_| SkillsRuntimeError::Unavailable)?;
        let is_dir = entry
            .file_type()
            .map_err(|_| SkillsRuntimeError::Unavailable)?
            .is_dir();
        let folder = entry.file_name().to_string_lossy().into_owned();
        if is_dir && valid_folder(&folder) {
            folders.push(folder);
        }
    }
    folders.sort();
    Ok(folders)
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
