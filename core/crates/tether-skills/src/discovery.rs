//! Skill discovery, safe loading, and prompt index rendering.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{SkillDocument, SkillError};

mod file_ops;
use file_ops::{
    checked_relative, count_skill_dirs, cut_chars, is_directory, is_symlink, map_io,
    read_skill_file, reference_files, validate_references,
};

const MAX_SKILLS: usize = 500;
pub(super) const MAX_SKILL_BYTES: u64 = 100 * 1024;
pub(super) const MAX_REFERENCE_BYTES: u64 = 256 * 1024;
pub(super) const MAX_READ_BYTES: usize = 64 * 1024;
pub(super) const ALLOWED_FOLDERS: [&str; 4] = ["references", "templates", "scripts", "assets"];

/// Where a discovered skill came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillSource {
    /// A skill in the trusted project.
    Project,
    /// A skill in the user's private skills directory.
    User,
    /// A skill shipped with the application.
    Bundled,
}

impl SkillSource {
    /// Returns the stable display label used by the tool result.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::User => "user",
            Self::Bundled => "bundled",
        }
    }
}

/// A bundled skill source supplied by the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundledSkill {
    /// The folder and public name of the skill.
    pub name: String,
    /// Directory containing `SKILL.md`.
    pub source: PathBuf,
}

/// Inputs used to discover project, user, and bundled skills.
#[derive(Clone, Copy)]
pub struct DiscoverInput<'a> {
    /// Project root, when a workspace is open.
    pub project_root: Option<&'a Path>,
    /// Whether project-controlled instructions may be loaded.
    pub project_trusted: bool,
    /// Private user skills directory.
    pub user_dir: &'a Path,
    /// Application-provided skills.
    pub bundled: &'a [BundledSkill],
}

/// One loaded skill and the metadata needed by the prompt index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    /// Public skill name.
    pub name: String,
    /// Trigger description from frontmatter.
    pub description: String,
    /// Where the skill was found.
    pub source: SkillSource,
    /// Skill directory.
    pub dir: PathBuf,
    /// Character count of the Markdown body.
    pub body_chars: usize,
}

/// A non-fatal discovery problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillIssue {
    /// Project skills were found before the project was trusted.
    ProjectSkillsNeedTrust {
        /// Number of project skill folders found.
        count: usize,
    },
    /// A lower-precedence skill was hidden by a winner.
    Shadowed {
        /// Hidden skill name.
        name: String,
        /// Source of the hidden skill.
        source: SkillSource,
        /// Source that won precedence.
        by: SkillSource,
    },
    /// A skill could not be parsed or safely read.
    Invalid {
        /// Path that caused the issue.
        path: PathBuf,
        /// Safe error detail.
        error: String,
    },
    /// A symbolic link was skipped.
    Symlink {
        /// Path that was skipped.
        path: PathBuf,
    },
    /// The discovery limit was reached.
    TooMany {
        /// Number of skills not loaded.
        count: usize,
    },
    /// A valid skill is disabled in settings.
    Disabled {
        /// Disabled skill name.
        name: String,
    },
}

/// Parsed skill data and files available for on-demand loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillView {
    /// Parsed frontmatter and body.
    pub document: SkillDocument,
    /// Safe relative reference paths.
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
struct Candidate {
    name: String,
    source: SkillSource,
    dir: PathBuf,
    document: SkillDocument,
}

/// The deterministic set of skills visible to an agent session.
#[derive(Debug, Clone)]
pub struct SkillsCatalog {
    /// Enabled skills in precedence order, then name order.
    pub skills: Vec<Skill>,
    /// Non-fatal discovery issues.
    pub issues: Vec<SkillIssue>,
    available: BTreeMap<String, Candidate>,
    disabled: BTreeSet<String>,
}

impl SkillsCatalog {
    /// Discovers skills without executing or following any skill-controlled file.
    #[must_use]
    pub fn discover(input: DiscoverInput<'_>) -> Self {
        let mut issues = Vec::new();
        let mut candidates = Vec::new();
        if let Some(root) = input.project_root {
            discover_project(root, input.project_trusted, &mut candidates, &mut issues);
        }
        discover_user(input.user_dir, &mut candidates, &mut issues);
        for bundled in input.bundled {
            candidates.extend(candidate_from_dir(
                &bundled.source,
                SkillSource::Bundled,
                &bundled.name,
                &mut issues,
            ));
        }
        select_candidates(candidates, issues)
    }

    /// Marks one skill disabled without removing it from the catalog listing.
    pub fn set_disabled(&mut self, name: &str, disabled: bool) {
        if disabled {
            self.disabled.insert(name.to_owned());
        } else {
            self.disabled.remove(name);
        }
        self.rebuild_enabled();
    }

    /// Returns every valid skill, including disabled skills, in stable order.
    #[must_use]
    pub fn listed(&self) -> Vec<Skill> {
        self.ordered_candidates()
            .into_iter()
            .map(skill_from_candidate)
            .collect()
    }

    /// Returns whether a skill is disabled.
    #[must_use]
    pub fn is_disabled(&self, name: &str) -> bool {
        self.disabled.contains(name)
    }

    /// Loads one enabled skill and lists its safe reference files.
    ///
    /// # Errors
    /// Returns an error when the skill is absent, disabled, or unsafe to read.
    pub fn view(&self, name: &str) -> Result<SkillView, SkillError> {
        if self.disabled.contains(name) {
            return Err(SkillError::NotFound);
        }
        let candidate = self.available.get(name).ok_or(SkillError::NotFound)?;
        let files =
            reference_files(&candidate.dir).map_err(|error| SkillError::Io(error.to_string()))?;
        Ok(SkillView {
            document: candidate.document.clone(),
            files,
        })
    }

    /// Reads a UTF-8 skill reference file with a bounded result.
    ///
    /// # Errors
    /// Returns an error for a missing, unsafe, non-UTF-8, or oversized path.
    pub fn read_file(&self, name: &str, relative: &str) -> Result<String, SkillError> {
        let candidate = self.available.get(name).ok_or(SkillError::NotFound)?;
        if self.disabled.contains(name) {
            return Err(SkillError::NotFound);
        }
        let path = checked_relative(&candidate.dir, relative)?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| map_io(&error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(SkillError::Symlink);
        }
        let bytes = fs::read(&path).map_err(|error| map_io(&error))?;
        let source = String::from_utf8(bytes).map_err(|_| SkillError::InvalidUtf8)?;
        Ok(cut_chars(&source, MAX_READ_BYTES))
    }

    fn rebuild_enabled(&mut self) {
        self.skills = self
            .ordered_candidates()
            .into_iter()
            .filter(|candidate| !self.disabled.contains(&candidate.name))
            .map(skill_from_candidate)
            .collect();
    }

    fn ordered_candidates(&self) -> Vec<&Candidate> {
        let mut candidates = self.available.values().collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| (candidate.source, candidate.name.clone()));
        candidates
    }
}

fn discover_project(
    root: &Path,
    trusted: bool,
    candidates: &mut Vec<Candidate>,
    issues: &mut Vec<SkillIssue>,
) {
    let project_dirs = [root.join(".tether/skills"), root.join(".agents/skills")];
    if trusted {
        for dir in project_dirs {
            discover_tree(&dir, SkillSource::Project, candidates, issues);
        }
        return;
    }
    let found = project_dirs.iter().map(|dir| count_skill_dirs(dir)).sum();
    if found > 0 {
        issues.push(SkillIssue::ProjectSkillsNeedTrust { count: found });
    }
}

fn select_candidates(candidates: Vec<Candidate>, mut issues: Vec<SkillIssue>) -> SkillsCatalog {
    let mut available: BTreeMap<String, Candidate> = BTreeMap::new();
    let mut sorted = candidates;
    sorted.sort_by_key(|candidate| {
        (
            candidate.source,
            candidate.name.clone(),
            candidate.dir.clone(),
        )
    });
    for candidate in sorted {
        if let Some(winner) = available.get(&candidate.name) {
            issues.push(SkillIssue::Shadowed {
                name: candidate.name,
                source: candidate.source,
                by: winner.source,
            });
            continue;
        }
        available.insert(candidate.name.clone(), candidate);
    }
    let overflow = available.len().saturating_sub(MAX_SKILLS);
    if overflow > 0 {
        issues.push(SkillIssue::TooMany { count: overflow });
        let keep: BTreeSet<String> = available.keys().take(MAX_SKILLS).cloned().collect();
        available.retain(|name, _| keep.contains(name));
    }
    let mut catalog = SkillsCatalog {
        skills: Vec::new(),
        issues,
        available,
        disabled: BTreeSet::new(),
    };
    catalog.rebuild_enabled();
    catalog
}

fn discover_user(root: &Path, candidates: &mut Vec<Candidate>, issues: &mut Vec<SkillIssue>) {
    discover_direct(root, SkillSource::User, candidates, issues);
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut categories = entries.filter_map(Result::ok).collect::<Vec<_>>();
    categories.sort_by_key(std::fs::DirEntry::file_name);
    for category in categories {
        let name = category.file_name();
        if name.to_string_lossy().starts_with('.') || !is_directory(&category.path()) {
            continue;
        }
        discover_direct(&category.path(), SkillSource::User, candidates, issues);
    }
}

fn discover_tree(
    root: &Path,
    source: SkillSource,
    candidates: &mut Vec<Candidate>,
    issues: &mut Vec<SkillIssue>,
) {
    discover_direct(root, source, candidates, issues);
}

fn discover_direct(
    root: &Path,
    source: SkillSource,
    candidates: &mut Vec<Candidate>,
    issues: &mut Vec<SkillIssue>,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if is_symlink(&path) {
            issues.push(SkillIssue::Symlink { path });
            continue;
        }
        if is_directory(&path) {
            candidates.extend(candidate_from_dir(&path, source, &name, issues));
        }
    }
}

fn candidate_from_dir(
    path: &Path,
    source: SkillSource,
    folder: &str,
    issues: &mut Vec<SkillIssue>,
) -> Option<Candidate> {
    let file = path.join("SKILL.md");
    let result = read_skill_file(&file, folder)
        .and_then(|document| validate_references(path, issues).map(|()| document));
    match result {
        Ok(document) => Some(Candidate {
            name: document.name.clone(),
            source,
            dir: path.to_owned(),
            document,
        }),
        Err(SkillError::Symlink) => {
            issues.push(SkillIssue::Symlink { path: file });
            None
        }
        Err(error) => {
            issues.push(SkillIssue::Invalid {
                path: file,
                error: format!("{error:?}"),
            });
            None
        }
    }
}

fn skill_from_candidate(candidate: &Candidate) -> Skill {
    Skill {
        name: candidate.document.name.clone(),
        description: candidate.document.description.clone(),
        source: candidate.source,
        dir: candidate.dir.clone(),
        body_chars: candidate.document.body.chars().count(),
    }
}
