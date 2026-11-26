//! Argument rules for the folder and file tools: the workspace root and sub folders.

use std::fs;
use std::path::PathBuf;

use tether_types::{SessionId, ToolCallId};
use tether_workspace::PathGuard;
use tokio_util::sync::CancellationToken;

use super::ToolLoop;
use super::registry::{ExecuteContext, PrepareContext};

fn fixture() -> (tempfile::TempDir, PathGuard) {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    fs::create_dir_all(workspace.join("sub")).unwrap();
    fs::write(workspace.join("note.txt"), "old").unwrap();
    fs::write(workspace.join("sub").join("inner.txt"), "inner").unwrap();
    let guard = PathGuard::new(&workspace).unwrap();
    (root, guard)
}

async fn list(loop_runner: &ToolLoop, guard: &PathGuard, args: &str) -> Result<String, String> {
    let call = loop_runner
        .registry
        .prepare("list_dir", args, &PrepareContext { guard })?;
    let session = SessionId::new("fixture");
    let call_id = ToolCallId::new("call");
    let mut before = |_: &[PathBuf]| -> Result<(), String> { Ok(()) };
    loop_runner
        .registry
        .execute(
            call,
            ExecuteContext {
                session_id: &session,
                tool_call_id: &call_id,
                cancel: CancellationToken::new(),
                sink: None,
                guard,
                before_edit: &mut before,
            },
        )
        .await
        .map(|output| output.text)
}

#[tokio::test]
async fn list_dir_lists_the_root_for_every_spelling() {
    let (_root, guard) = fixture();
    let loop_runner = ToolLoop::new(guard.clone());
    for args in [
        "{}",
        r#"{"path":"."}"#,
        r#"{"path":""}"#,
        r#"{"path":"./"}"#,
    ] {
        let text = list(&loop_runner, &guard, args).await.unwrap();
        assert!(text.contains("note.txt"), "{args}: {text}");
        assert!(text.contains("sub/"), "{args}: {text}");
    }
}

#[tokio::test]
async fn list_dir_lists_a_named_folder_for_every_spelling() {
    let (_root, guard) = fixture();
    let loop_runner = ToolLoop::new(guard.clone());
    for args in [
        r#"{"path":"sub"}"#,
        r#"{"path":"./sub"}"#,
        r#"{"path":"sub/"}"#,
    ] {
        let text = list(&loop_runner, &guard, args).await.unwrap();
        assert!(text.contains("inner.txt"), "{args}: {text}");
        assert!(!text.contains("note.txt"), "{args}: {text}");
    }
}

#[test]
fn list_dir_rejects_escapes_and_non_string_paths() {
    let (_root, guard) = fixture();
    let loop_runner = ToolLoop::new(guard.clone());
    for args in [r#"{"path":".."}"#, r#"{"path":"/etc"}"#] {
        let error = loop_runner
            .registry
            .prepare("list_dir", args, &PrepareContext { guard: &guard })
            .unwrap_err();
        assert_eq!(error, "path leaves the workspace", "{args}");
    }
    let error = loop_runner
        .registry
        .prepare(
            "list_dir",
            r#"{"path":12}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap_err();
    assert_eq!(error, "path must be a string");
}
