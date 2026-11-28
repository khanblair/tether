//! Sample skills bundled with the application.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path};

use crate::SkillError;

/// One file embedded in a sample skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleFile {
    /// Path relative to the sample skill folder.
    pub path: &'static str,
    /// UTF-8 file contents.
    pub contents: &'static str,
}

/// One sample skill and all of its files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleSkill {
    /// Skill folder and public name.
    pub name: &'static str,
    /// Files relative to the skill folder.
    pub files: &'static [SampleFile],
}

/// The result of installing a sample skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleInstall {
    /// The sample was written to the user skills folder.
    Installed,
    /// A file, folder, or symbolic link already used the sample name.
    AlreadyInstalled,
}

static CAVEMAN_FILES: [SampleFile; 1] = [SampleFile {
    path: "SKILL.md",
    contents: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/samples/caveman/SKILL.md"
    )),
}];
static CODE_REVIEW_FILES: [SampleFile; 1] = [SampleFile {
    path: "SKILL.md",
    contents: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/samples/code-review/SKILL.md"
    )),
}];
static DEBUG_FILES: [SampleFile; 1] = [SampleFile {
    path: "SKILL.md",
    contents: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/samples/debug/SKILL.md"
    )),
}];
static DIAGNOSING_BUGS_FILES: [SampleFile; 2] = [
    SampleFile {
        path: "SKILL.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/samples/diagnosing-bugs/SKILL.md"
        )),
    },
    SampleFile {
        path: "scripts/hitl-loop.template.sh",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/samples/diagnosing-bugs/scripts/hitl-loop.template.sh"
        )),
    },
];
static HANDOFF_FILES: [SampleFile; 1] = [SampleFile {
    path: "SKILL.md",
    contents: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/samples/handoff/SKILL.md"
    )),
}];
static RESEARCH_FILES: [SampleFile; 1] = [SampleFile {
    path: "SKILL.md",
    contents: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/samples/research/SKILL.md"
    )),
}];
static RESOLVING_MERGE_CONFLICTS_FILES: [SampleFile; 1] = [SampleFile {
    path: "SKILL.md",
    contents: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/samples/resolving-merge-conflicts/SKILL.md"
    )),
}];
static TDD_FILES: [SampleFile; 3] = [
    SampleFile {
        path: "SKILL.md",
        contents: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/tdd/SKILL.md")),
    },
    SampleFile {
        path: "mocking.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/samples/tdd/mocking.md"
        )),
    },
    SampleFile {
        path: "tests.md",
        contents: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/tdd/tests.md")),
    },
];
static TO_SPEC_FILES: [SampleFile; 1] = [SampleFile {
    path: "SKILL.md",
    contents: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/samples/to-spec/SKILL.md"
    )),
}];
static TRIAGE_FILES: [SampleFile; 3] = [
    SampleFile {
        path: "AGENT-BRIEF.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/samples/triage/AGENT-BRIEF.md"
        )),
    },
    SampleFile {
        path: "OUT-OF-SCOPE.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/samples/triage/OUT-OF-SCOPE.md"
        )),
    },
    SampleFile {
        path: "SKILL.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/samples/triage/SKILL.md"
        )),
    },
];
static WRITING_FOR_AGENTS_FILES: [SampleFile; 2] = [
    SampleFile {
        path: "SKILL-MECHANICS.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/samples/writing-for-agents/SKILL-MECHANICS.md"
        )),
    },
    SampleFile {
        path: "SKILL.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/samples/writing-for-agents/SKILL.md"
        )),
    },
];

static SAMPLE_SKILLS: [SampleSkill; 11] = [
    SampleSkill {
        name: "caveman",
        files: &CAVEMAN_FILES,
    },
    SampleSkill {
        name: "code-review",
        files: &CODE_REVIEW_FILES,
    },
    SampleSkill {
        name: "debug",
        files: &DEBUG_FILES,
    },
    SampleSkill {
        name: "diagnosing-bugs",
        files: &DIAGNOSING_BUGS_FILES,
    },
    SampleSkill {
        name: "handoff",
        files: &HANDOFF_FILES,
    },
    SampleSkill {
        name: "research",
        files: &RESEARCH_FILES,
    },
    SampleSkill {
        name: "resolving-merge-conflicts",
        files: &RESOLVING_MERGE_CONFLICTS_FILES,
    },
    SampleSkill {
        name: "tdd",
        files: &TDD_FILES,
    },
    SampleSkill {
        name: "to-spec",
        files: &TO_SPEC_FILES,
    },
    SampleSkill {
        name: "triage",
        files: &TRIAGE_FILES,
    },
    SampleSkill {
        name: "writing-for-agents",
        files: &WRITING_FOR_AGENTS_FILES,
    },
];

/// Returns all embedded sample skills in name order.
#[must_use]
pub fn sample_skills() -> &'static [SampleSkill] {
    &SAMPLE_SKILLS
}

/// Returns the bundled attribution notice and license texts.
#[must_use]
pub fn sample_notice() -> String {
    let mut notice: String =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/NOTICE.md")).into();
    for (name, text) in [
        (
            "caveman-LICENSE-Apache-2.0.txt",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/samples/licenses/caveman-LICENSE-Apache-2.0.txt"
            )),
        ),
        (
            "caveman-NOTICE.txt",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/samples/licenses/caveman-NOTICE.txt"
            )),
        ),
        (
            "javascript-mastery-skills-LICENSE-MIT.txt",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/samples/licenses/javascript-mastery-skills-LICENSE-MIT.txt"
            )),
        ),
        (
            "mattpocock-skills-LICENSE-MIT.txt",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/samples/licenses/mattpocock-skills-LICENSE-MIT.txt"
            )),
        ),
    ] {
        notice.push_str("\n## ");
        notice.push_str(name);
        notice.push('\n');
        notice.push_str(text);
    }
    notice
}

/// Installs one embedded sample skill into a user skills directory.
///
/// # Errors
/// Returns `NotFound` for an unknown sample, `AlreadyInstalled` for an occupied name, or an
/// installation error when the directory cannot be created or written.
pub fn install_sample(skills_dir: &Path, name: &str) -> Result<SampleInstall, SkillError> {
    let sample = sample_skills()
        .iter()
        .find(|sample| sample.name == name)
        .ok_or(SkillError::NotFound)?;
    for file in sample.files {
        validate_sample_path(file.path)?;
    }

    let target = skills_dir.join(sample.name);
    match fs::symlink_metadata(&target) {
        Ok(_) => return Ok(SampleInstall::AlreadyInstalled),
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(io_error(error)),
        Err(_) => {}
    }
    fs::create_dir_all(skills_dir).map_err(io_error)?;
    let staging = skills_dir.join(format!(".install-{}-{}", sample.name, std::process::id()));
    fs::create_dir(&staging).map_err(io_error)?;

    let result = write_sample(sample, &staging)
        .and_then(|()| fs::rename(&staging, &target).map_err(io_error));
    match result {
        Ok(()) => Ok(SampleInstall::Installed),
        Err(error) => {
            drop(fs::remove_dir_all(&staging));
            Err(error)
        }
    }
}

fn write_sample(sample: &SampleSkill, staging: &Path) -> Result<(), SkillError> {
    for file in sample.files {
        validate_sample_path(file.path)?;
        let destination = staging.join(file.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(io_error)?;
        output
            .write_all(file.contents.as_bytes())
            .map_err(io_error)?;
    }
    Ok(())
}

fn validate_sample_path(path: &str) -> Result<(), SkillError> {
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(SkillError::InvalidPath);
    }
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err passes owned I/O errors"
)]
fn io_error(error: io::Error) -> SkillError {
    SkillError::Io(error.to_string())
}

#[cfg(test)]
#[path = "samples_tests.rs"]
mod samples_tests;
