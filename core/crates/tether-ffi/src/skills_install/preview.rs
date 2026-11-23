//! Quarantine preview: extracts a skill ZIP, inspects it, and keeps it until approval.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use tether_skills::{ScanReport, ScanVerdict, SkillFile, parse_skill, scan_skill};
use tether_workspace::archive::{ArchiveError, extract_zip};

use super::LIMITS;
use super::NEXT_ID;
use super::download::SkillDownloader;
use super::tree::{find_skill_dir, read_files};
use super::views::{QuarantineFileView, QuarantineView};
use crate::skills::{SkillsRuntime, SkillsRuntimeError};

/// A skill folder read from quarantine content, with its scan.
pub(super) struct Inspected {
    pub(super) skill_dir: PathBuf,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) files: Vec<(String, Vec<u8>)>,
    pub(super) scan: ScanReport,
}

impl SkillsRuntime {
    pub(super) fn preview_with(
        &self,
        url: String,
        downloader: Arc<dyn SkillDownloader>,
        label: String,
    ) -> Result<QuarantineView, SkillsRuntimeError> {
        let bytes = std::thread::spawn(move || downloader.download(&url))
            .join()
            .map_err(|_| SkillsRuntimeError::Download)??;
        self.preview_bytes(&bytes, label)
    }

    pub(super) fn preview_bytes(
        &self,
        bytes: &[u8],
        label: String,
    ) -> Result<QuarantineView, SkillsRuntimeError> {
        self.expire_quarantine((self.clock)())?;
        let id = format!(
            "{}-{}",
            (self.clock)(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        );
        let id_dir = self.quarantine_dir(&id)?;
        fs::create_dir_all(&id_dir).map_err(|_| SkillsRuntimeError::Unavailable)?;
        let result = self.extract_preview(&id, &id_dir, bytes, label);
        if result.is_err() {
            drop(fs::remove_dir_all(id_dir));
        }
        result
    }

    fn extract_preview(
        &self,
        id: &str,
        id_dir: &Path,
        bytes: &[u8],
        label: String,
    ) -> Result<QuarantineView, SkillsRuntimeError> {
        fs::write(id_dir.join("created"), (self.clock)().to_string())
            .map_err(|_| SkillsRuntimeError::Unavailable)?;
        let zip_path = id_dir.join("source.zip");
        let content = id_dir.join("content");
        fs::write(&zip_path, bytes).map_err(|_| SkillsRuntimeError::Unavailable)?;
        fs::create_dir(&content).map_err(|_| SkillsRuntimeError::Unavailable)?;
        let report = extract_zip(&zip_path, &content, &LIMITS, &|| false, true)
            .map_err(|error| map_archive_error(&error))?;
        drop(fs::remove_file(zip_path));
        if report.skipped_symlinks > 0 {
            return Err(SkillsRuntimeError::NotASkill);
        }
        let inspected = inspect_content(&content)?;
        Ok(QuarantineView {
            id: id.into(),
            name: inspected.name,
            description: inspected.description,
            files: inspected.files.into_iter().map(file_view).collect(),
            can_install: inspected.scan.verdict != ScanVerdict::Blocked,
            scan: inspected.scan.into(),
            source_label: label,
        })
    }
}

/// Finds the skill folder under quarantine content, parses it, and scans its files.
pub(super) fn inspect_content(content: &Path) -> Result<Inspected, SkillsRuntimeError> {
    let skill_dir = find_skill_dir(content)?;
    let source = fs::read_to_string(skill_dir.join("SKILL.md"))
        .map_err(|_| SkillsRuntimeError::NotASkill)?;
    let folder = skill_folder_name(content, &skill_dir, &source)?;
    let document = parse_skill(&folder, &source).map_err(|_| SkillsRuntimeError::NotASkill)?;
    let files = read_files(&skill_dir)?;
    let scan_files = files
        .iter()
        .map(|(path, bytes)| SkillFile {
            path: format!("{folder}/{path}"),
            bytes: bytes.clone(),
        })
        .collect::<Vec<_>>();
    let scan = scan_skill(&scan_files);
    Ok(Inspected {
        skill_dir,
        name: document.name,
        description: document.description,
        files,
        scan,
    })
}

/// The folder name the skill is judged against.
///
/// A skill in a sub folder must be named like that folder. A skill whose `SKILL.md` sits at the
/// top of the archive has no folder of its own (the extraction folder is a temporary name), so
/// it is judged against the name it declares; the install creates a folder with that name.
fn skill_folder_name(
    content: &Path,
    skill_dir: &Path,
    source: &str,
) -> Result<String, SkillsRuntimeError> {
    if skill_dir != content {
        return skill_dir
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or(SkillsRuntimeError::NotASkill);
    }
    declared_name(source).ok_or(SkillsRuntimeError::NotASkill)
}

/// Reads the `name:` value from the front matter block at the top of a `SKILL.md`.
fn declared_name(source: &str) -> Option<String> {
    let mut lines = source.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    lines
        .take_while(|line| line.trim() != "---")
        .find_map(|line| line.strip_prefix("name:"))
        .map(|value| value.trim().trim_matches('"').trim_matches('\'').to_owned())
}

fn map_archive_error(error: &ArchiveError) -> SkillsRuntimeError {
    match error {
        ArchiveError::TooLarge | ArchiveError::TooManyEntries | ArchiveError::Ratio => {
            SkillsRuntimeError::TooLarge
        }
        ArchiveError::Io(_) => SkillsRuntimeError::Unavailable,
        _ => SkillsRuntimeError::NotASkill,
    }
}

fn file_view((path, bytes): (String, Vec<u8>)) -> QuarantineFileView {
    QuarantineFileView {
        is_script: [".sh", ".py", ".js", ".ts", ".rb"]
            .iter()
            .any(|suffix| path.ends_with(suffix)),
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        path,
    }
}
