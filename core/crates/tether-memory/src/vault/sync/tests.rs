use super::{SyncManifest, VaultFolder, sync_folders};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;

#[derive(Default)]
struct FakeFolder { files: RefCell<BTreeMap<String, Vec<u8>>> }

impl FakeFolder {
    fn set(&self, path: &str, body: &str) {
        self.files.borrow_mut().insert(path.into(), body.as_bytes().to_vec());
    }
    fn text(&self, path: &str) -> Option<String> {
        self.files.borrow().get(path).map(|bytes| String::from_utf8(bytes.clone()).unwrap())
    }
}

impl VaultFolder for FakeFolder {
    fn list(&self) -> io::Result<Vec<String>> { Ok(self.files.borrow().keys().cloned().collect()) }
    fn read(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        Ok(self.files.borrow().get(path).cloned())
    }
    fn write(&self, path: &str, bytes: &[u8]) -> io::Result<()> {
        self.files.borrow_mut().insert(path.into(), bytes.to_vec());
        Ok(())
    }
}

#[test]
fn one_sided_edits_copy_in_both_directions() {
    let private = FakeFolder::default();
    let shared = FakeFolder::default();
    let mut manifest = SyncManifest::default();
    private.set("Note.md", "first");
    let first = sync_folders(&private, &shared, &mut manifest).unwrap();
    assert_eq!(first.copied_to_shared, vec!["Note.md"]);
    shared.set("Note.md", "from shared");
    let second = sync_folders(&private, &shared, &mut manifest).unwrap();
    assert_eq!(second.copied_to_private, vec!["Note.md"]);
    assert_eq!(private.text("Note.md").as_deref(), Some("from shared"));
}

#[test]
fn simultaneous_edits_keep_both_originals_and_two_named_copies() {
    let private = FakeFolder::default();
    let shared = FakeFolder::default();
    let mut manifest = SyncManifest::default();
    private.set("Note.md", "base");
    sync_folders(&private, &shared, &mut manifest).unwrap();
    private.set("Note.md", "local edit");
    shared.set("Note.md", "shared edit");
    let report = sync_folders(&private, &shared, &mut manifest).unwrap();
    assert_eq!(report.conflicts, vec!["Note.md"]);
    assert_eq!(private.text("Note.md").as_deref(), Some("local edit"));
    assert_eq!(shared.text("Note.md").as_deref(), Some("shared edit"));
    assert_eq!(private.text("Note.conflict-shared.md").as_deref(), Some("shared edit"));
    assert_eq!(shared.text("Note.conflict-private.md").as_deref(), Some("local edit"));
    assert!(sync_folders(&private, &shared, &mut manifest).unwrap().conflicts.is_empty());
}
