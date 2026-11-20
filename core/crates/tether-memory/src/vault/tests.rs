use super::{VaultError, VaultIndex};
use std::fs;
use std::path::PathBuf;

struct VaultFixture {
    path: PathBuf,
}

impl VaultFixture {
    fn new() -> Self {
        let stamp = crate::unique_id();
        let path = std::env::temp_dir().join(format!("tether-vault-{stamp}"));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.path.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

impl Drop for VaultFixture {
    fn drop(&mut self) {
        // Best-effort cleanup. A panic in Drop would hide the test result.
        fs::remove_dir_all(&self.path).ok();
    }
}

#[test]
fn rebuild_is_deterministic_and_marks_missing_links() {
    let fixture = VaultFixture::new();
    fixture.write("Work.md", "---\ndescription: Work map\ntype: moc\ntags: [work, active]\n---\n# Tasks\n[[Plan#Next|next step]] and [[Missing]]");
    fixture.write(
        "Plan.md",
        "---\ndescription: Current plan\n---\n## Next\nDetails",
    );
    let first = VaultIndex::rebuild(&fixture.path).unwrap();
    assert_eq!(first, VaultIndex::rebuild(&fixture.path).unwrap());
    assert_eq!(first.notes["Work.md"].tags, vec!["work", "active"]);
    assert_eq!(first.notes["Plan.md"].headings, vec!["Next"]);
    assert_eq!(first.links.len(), 2);
    assert!(first.links[0].resolved);
    assert_eq!(first.links[0].heading.as_deref(), Some("Next"));
    assert_eq!(first.links[0].alias.as_deref(), Some("next step"));
    assert!(!first.links[1].resolved);
    assert_eq!(first.backlinks("Plan.md").len(), 1);
    fs::remove_file(fixture.path.join("Plan.md")).unwrap();
    let rebuilt = VaultIndex::rebuild(&fixture.path).unwrap();
    assert!(!rebuilt.links[0].resolved);
}

#[test]
fn note_without_description_is_rejected() {
    let fixture = VaultFixture::new();
    fixture.write("NoDescription.md", "---\ntype: moc\n---\n# Note");
    assert!(matches!(
        VaultIndex::rebuild(&fixture.path),
        Err(VaultError::InvalidNote { .. })
    ));
}
