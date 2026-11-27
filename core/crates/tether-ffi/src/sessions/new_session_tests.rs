use std::fs;
use std::sync::Arc;

use super::super::*;
use crate::{EventSink, NewSessionPreviewView, SecretVault, SecretsPlatform};
use tether_store::Store;
use tether_types::{ModelRef, PermissionMode, Provider};

struct PlainPlatform;

impl SecretsPlatform for PlainPlatform {
    fn wrap(
        &self,
        plaintext: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, crate::SecretsPlatformFailure> {
        Ok(plaintext)
    }

    fn unwrap(
        &self,
        wrapped: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, crate::SecretsPlatformFailure> {
        Ok(wrapped)
    }
}

struct NoopSink;

impl EventSink for NoopSink {
    fn on_event(&self, _event: crate::CoreEventView) {}
}

struct Fixture {
    dir: tempfile::TempDir,
    runtime: Arc<SessionRuntime>,
    project: tether_store::ProjectRecord,
    workspace: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("tether.db");
        let mut store = Store::open(&db).unwrap();
        let project = store.create_project("Project", 1).unwrap();
        let workspace = dir.path().join("workspaces").join(&project.path);
        fs::create_dir_all(&workspace).unwrap();
        let vault = SecretVault::new(
            dir.path().join("secrets").to_string_lossy().into_owned(),
            Arc::new(PlainPlatform),
        );
        let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
        Self {
            dir,
            runtime,
            project,
            workspace,
        }
    }

    fn preview(&self) -> NewSessionPreviewView {
        self.runtime
            .new_session_preview(self.project.id.as_str().to_owned())
            .unwrap()
    }
}

fn write_skill(root: &std::path::Path, name: &str, valid: bool) {
    let folder = root.join(name);
    fs::create_dir_all(&folder).unwrap();
    let content = if valid {
        format!("---\nname: {name}\ndescription: A test skill.\n---\nBody")
    } else {
        "not frontmatter".to_owned()
    };
    fs::write(folder.join("SKILL.md"), content).unwrap();
}

#[test]
fn preview_reads_agents_instructions_and_counts_lines() {
    let fixture = Fixture::new();
    fs::write(fixture.workspace.join("AGENTS.md"), "one\ntwo\nthree").unwrap();

    let preview = fixture.preview();

    assert_eq!(preview.instructions_file.as_deref(), Some("AGENTS.md"));
    assert_eq!(preview.instructions_lines, 3);
}

#[test]
fn preview_prefers_tether_instructions() {
    let fixture = Fixture::new();
    fs::write(fixture.workspace.join("TETHER.md"), "one").unwrap();
    fs::write(fixture.workspace.join("CLAUDE.md"), "one\ntwo").unwrap();

    assert_eq!(
        fixture.preview().instructions_file.as_deref(),
        Some("TETHER.md")
    );
}

#[test]
fn preview_has_no_instructions_when_the_project_has_none() {
    let fixture = Fixture::new();

    let preview = fixture.preview();

    assert_eq!(preview.instructions_file, None);
    assert_eq!(preview.instructions_lines, 0);
}

#[test]
fn preview_counts_lines_after_the_instruction_cap() {
    let fixture = Fixture::new();
    fs::write(fixture.workspace.join("AGENTS.md"), "line\n".repeat(20_000)).unwrap();

    let preview = fixture.preview();

    assert!(preview.instructions_lines > 0);
    assert!(preview.instructions_lines < 20_000);
}

#[test]
fn preview_counts_memory_entries_from_both_files() {
    let fixture = Fixture::new();
    let memory = fixture.dir.path().join("memory");
    fs::create_dir_all(&memory).unwrap();
    fs::write(memory.join("MEMORY.md"), "one\n§\n\n§\n two\n§\nthree").unwrap();
    fs::write(memory.join("USER.md"), "four\n§\n\n§\n five").unwrap();

    assert_eq!(fixture.preview().memory_entries, 5);
}

#[test]
fn preview_counts_only_valid_enabled_skills() {
    let fixture = Fixture::new();
    let skills = fixture.dir.path().join("skills");
    write_skill(&skills, "valid-skill", true);
    write_skill(&skills, "invalid-skill", false);
    write_skill(&skills, "disabled-skill", true);
    Store::open(&fixture.dir.path().join("tether.db"))
        .unwrap()
        .set_skill_enabled("disabled-skill", false)
        .unwrap();

    assert_eq!(fixture.preview().skills_available, 1);
}

#[test]
fn preview_returns_newest_non_archived_session() {
    let fixture = Fixture::new();
    let db = fixture.dir.path().join("tether.db");
    let mut store = Store::open(&db).unwrap();
    let old = store
        .create_session(
            &fixture.project.id,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "model".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap();
    store.rename_session(&old.id, "Kept session").unwrap();
    let archived = store
        .create_session(
            &fixture.project.id,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "model".into(),
            },
            PermissionMode::Ask,
            20,
        )
        .unwrap();
    store
        .rename_session(&archived.id, "Archived session")
        .unwrap();
    store.set_session_archived(&archived.id, true).unwrap();

    let preview = fixture.preview();

    assert_eq!(preview.last_session_title.as_deref(), Some("Kept session"));
    assert_eq!(preview.last_session_updated_at_ms, Some(old.updated_at));
}

#[test]
fn preview_has_no_last_session_when_the_project_is_empty() {
    let fixture = Fixture::new();

    assert_eq!(fixture.preview().last_session_title, None);
}

#[test]
fn preview_rejects_an_unknown_project() {
    let fixture = Fixture::new();

    assert!(matches!(
        fixture.runtime.new_session_preview("missing".into()),
        Err(SessionRuntimeError::NotFound)
    ));
}

#[cfg(unix)]
#[test]
fn preview_does_not_read_an_instruction_symlink() {
    let fixture = Fixture::new();
    let outside = fixture.dir.path().join("outside.md");
    fs::write(&outside, "outside").unwrap();
    std::os::unix::fs::symlink(&outside, fixture.workspace.join("AGENTS.md")).unwrap();

    let preview = fixture.preview();

    assert_eq!(preview.instructions_file, None);
    assert_eq!(preview.instructions_lines, 0);
}
