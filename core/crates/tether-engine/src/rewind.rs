//! Restore a saved edit checkpoint.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use tether_store::{BlobStore, Checkpoint, Store, StoreError};
use tether_types::SessionId;
use tether_workspace::PathGuard;
use thiserror::Error;

/// Which state to restore from an edit checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewindScope {
    /// Restore the workspace files only.
    Files,
    /// Hide the checkpoint's assistant call and later messages only.
    Conversation,
    /// Restore both files and conversation.
    Both,
}

/// A checkpoint could not be restored.
#[derive(Debug, Error)]
pub enum RewindError {
    /// Snapshot or transcript storage failed.
    #[error("rewind store failed: {0}")]
    Store(#[from] StoreError),
    /// A workspace path or file operation failed.
    #[error("rewind file failed: {0}")]
    Io(#[from] io::Error),
    /// A conversation rewind needs a saved assistant message.
    #[error("checkpoint has no conversation anchor")]
    NoMessage,
}

/// Restores the selected state immediately before a checkpoint's edit batch.
/// Later file snapshots are applied from newest to oldest. The caller must
/// serialize this operation with turns in the session actor.
///
/// # Errors
/// Returns an error for an absent checkpoint, invalid path, corrupt blob, or write failure.
#[expect(
    clippy::too_many_arguments,
    reason = "the checkpoint, scope, store, and guarded workspace are explicit"
)]
pub fn rewind_checkpoint(
    store: &mut Store,
    session_id: &SessionId,
    checkpoint_id: &str,
    scope: RewindScope,
    guard: &PathGuard,
    blobs: &BlobStore,
) -> Result<(), RewindError> {
    let chain = store.checkpoint_chain(session_id, checkpoint_id)?;
    let selected = chain.last().ok_or(StoreError::MissingCheckpoint)?;
    let anchor = if scope == RewindScope::Files {
        None
    } else {
        Some(selected.message_id.as_ref().ok_or(RewindError::NoMessage)?)
    };
    if let Some(message_id) = anchor {
        store.validate_rewind_anchor(session_id, message_id)?;
    }
    if scope != RewindScope::Conversation {
        for (target, bytes) in file_plan(&chain, guard, blobs)? {
            restore_file(&target, bytes.as_deref())?;
        }
    }
    if let Some(message_id) = anchor {
        store.rewind_conversation(session_id, message_id)?;
    }
    Ok(())
}

/// Hides the checkpoint's assistant call and later messages without opening a workspace.
///
/// # Errors
/// Returns an error when the checkpoint or its conversation anchor is absent.
pub fn rewind_conversation_only(
    store: &mut Store,
    session_id: &SessionId,
    checkpoint_id: &str,
) -> Result<(), RewindError> {
    let chain = store.checkpoint_chain(session_id, checkpoint_id)?;
    let selected = chain.last().ok_or(StoreError::MissingCheckpoint)?;
    let anchor = selected.message_id.as_ref().ok_or(RewindError::NoMessage)?;
    store.rewind_conversation(session_id, anchor)?;
    Ok(())
}

fn file_plan(
    chain: &[Checkpoint],
    guard: &PathGuard,
    blobs: &BlobStore,
) -> Result<BTreeMap<PathBuf, Option<Vec<u8>>>, RewindError> {
    let mut plan = BTreeMap::new();
    for checkpoint in chain {
        for file in &checkpoint.files {
            let target = checked_target(guard, &file.path)?;
            let bytes = file
                .blob_hash
                .as_deref()
                .map(|hash| blobs.get(hash))
                .transpose()?;
            plan.insert(target, bytes);
        }
    }
    Ok(plan)
}

fn checked_target(guard: &PathGuard, relative: &Path) -> io::Result<PathBuf> {
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(name) if name != ".git"))
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "invalid snapshot path",
        ));
    }
    guard.resolve(relative)
}

fn restore_file(target: &Path, bytes: Option<&[u8]>) -> io::Result<()> {
    match bytes {
        Some(bytes) => {
            let parent = target.parent().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "snapshot has no parent")
            })?;
            fs::create_dir_all(parent)?;
            let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
            temporary.write_all(bytes)?;
            temporary.as_file().sync_all()?;
            temporary.persist(target).map_err(|error| error.error)?;
            Ok(())
        }
        None => match fs::remove_file(target) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        },
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::excessive_nesting,
        reason = "the actor fixture follows turn and rewind events"
    )]
    use std::fs;
    use std::sync::Arc;

    use tether_providers::{BoxFuture, ChatProvider, ChatRequest, ProviderError, StreamEvent};
    use tether_store::{BlobStore, SnapshotInput, Store};
    use tether_types::{CoreEvent, MessagePart, Role, SessionId};
    use tether_workspace::PathGuard;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::{RewindScope, rewind_checkpoint};
    use crate::{
        SessionActor, ToolRuntime,
        session_tests::{Recorder, next_event},
    };

    struct UntilCancelled;

    impl ChatProvider for UntilCancelled {
        fn stream_chat<'a>(
            &'a self,
            _request: &'a ChatRequest,
            _events: mpsc::Sender<StreamEvent>,
            cancel: CancellationToken,
        ) -> BoxFuture<'a, Result<(), ProviderError>> {
            Box::pin(async move {
                cancel.cancelled().await;
                Err(ProviderError::Cancelled)
            })
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "fixture contains two edits and their checkpoint states"
    )]
    fn fixture() -> (tempfile::TempDir, Store, SessionId, String, BlobStore) {
        let root = tempfile::tempdir().unwrap();
        let database = root.path().join("store.db");
        drop(Store::open(&database).unwrap());
        rusqlite::Connection::open(&database).unwrap().execute_batch(
            "INSERT INTO projects (id, name, path, created_at) VALUES ('project', 'fixture', 'fixture', 1);
             INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
             VALUES ('session', 'project', 'openai', 'fixture', 'ask', 'idle', 1, 1);",
        ).unwrap();
        let mut store = Store::open(&database).unwrap();
        let session = SessionId::new("session");
        let workspace = root.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        fs::write(workspace.join("note.txt"), b"original").unwrap();
        store.begin_turn(&session, "change it", 2).unwrap();
        let assistant = store
            .append_message(
                &session,
                Role::Assistant,
                &[MessagePart::Text {
                    text: "editing".into(),
                }],
                3,
            )
            .unwrap();
        let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
        let first = store
            .save_checkpoint(
                &session,
                Some(&assistant.id),
                &blobs,
                &[
                    SnapshotInput {
                        path: "note.txt".into(),
                        content: Some(b"original".to_vec()),
                    },
                    SnapshotInput {
                        path: "new.txt".into(),
                        content: None,
                    },
                ],
                4,
            )
            .unwrap();
        fs::write(workspace.join("note.txt"), b"middle").unwrap();
        fs::write(workspace.join("new.txt"), b"created").unwrap();
        let later = store
            .append_message(
                &session,
                Role::Tool,
                &[MessagePart::Text {
                    text: "done".into(),
                }],
                5,
            )
            .unwrap();
        store
            .save_checkpoint(
                &session,
                Some(&later.id),
                &blobs,
                &[SnapshotInput {
                    path: "note.txt".into(),
                    content: Some(b"middle".to_vec()),
                }],
                6,
            )
            .unwrap();
        fs::write(workspace.join("note.txt"), b"latest").unwrap();
        (root, store, session, first.id, blobs)
    }

    #[test]
    fn each_rewind_scope_restores_only_its_selected_state() {
        for scope in [
            RewindScope::Files,
            RewindScope::Conversation,
            RewindScope::Both,
        ] {
            let (root, mut store, session, checkpoint, blobs) = fixture();
            let workspace = root.path().join("workspace");
            let guard = PathGuard::new(&workspace).unwrap();
            rewind_checkpoint(&mut store, &session, &checkpoint, scope, &guard, &blobs).unwrap();
            let files_restored = scope != RewindScope::Conversation;
            assert_eq!(
                fs::read(workspace.join("note.txt")).unwrap(),
                if files_restored {
                    b"original".as_slice()
                } else {
                    b"latest".as_slice()
                }
            );
            assert_eq!(workspace.join("new.txt").exists(), !files_restored);
            let messages = store.messages(&session).unwrap();
            let conversation_rewound = scope != RewindScope::Files;
            assert!(!messages[0].rewound);
            assert_eq!(messages[1].rewound, conversation_rewound);
            assert_eq!(messages[2].rewound, conversation_rewound);
        }
    }

    #[test]
    fn corrupt_blob_aborts_before_changing_any_file() {
        let (root, mut store, session, checkpoint, blobs) = fixture();
        let saved = store.load_checkpoint(&checkpoint).unwrap().unwrap();
        let hash = saved
            .files
            .iter()
            .find_map(|file| file.blob_hash.as_deref())
            .unwrap();
        fs::write(
            root.path().join("blobs").join(&hash[..2]).join(hash),
            b"damaged",
        )
        .unwrap();
        let workspace = root.path().join("workspace");
        let guard = PathGuard::new(&workspace).unwrap();
        assert!(
            rewind_checkpoint(
                &mut store,
                &session,
                &checkpoint,
                RewindScope::Both,
                &guard,
                &blobs
            )
            .is_err()
        );
        assert_eq!(fs::read(workspace.join("note.txt")).unwrap(), b"latest");
        assert!(workspace.join("new.txt").exists());
        assert!(
            store
                .messages(&session)
                .unwrap()
                .iter()
                .all(|message| !message.rewound)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_restore_target_is_rejected_without_following_it() {
        use std::os::unix::fs::symlink;

        let (root, mut store, session, checkpoint, blobs) = fixture();
        let workspace = root.path().join("workspace");
        let outside = root.path().join("outside.txt");
        fs::write(&outside, b"private").unwrap();
        fs::remove_file(workspace.join("note.txt")).unwrap();
        symlink(&outside, workspace.join("note.txt")).unwrap();
        let guard = PathGuard::new(&workspace).unwrap();
        assert!(
            rewind_checkpoint(
                &mut store,
                &session,
                &checkpoint,
                RewindScope::Files,
                &guard,
                &blobs
            )
            .is_err()
        );
        assert_eq!(fs::read(outside).unwrap(), b"private");
        assert!(workspace.join("note.txt").is_symlink());
    }

    #[tokio::test]
    async fn actor_cancels_a_turn_before_rewind_and_emits_reload_event() {
        let (root, store, session, checkpoint, blobs) = fixture();
        let workspace = root.path().join("workspace");
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (actor, handle) = SessionActor::new_with_tool_runtime(
            store,
            &session,
            Arc::new(|| Ok(Arc::new(UntilCancelled))),
            Arc::new(Recorder(tx)),
            ToolRuntime {
                guard: PathGuard::new(&workspace).unwrap(),
                blobs,
                approve: Arc::new(|_, _| false),
            },
        )
        .unwrap();
        let task = tokio::spawn(actor.run());
        handle.send_message("new request".into()).await.unwrap();
        loop {
            if matches!(next_event(&mut rx).await, CoreEvent::SessionUpdated { .. }) {
                break;
            }
        }
        handle.rewind(checkpoint, RewindScope::Both).await.unwrap();
        let mut saw_reload = false;
        while let Ok(event) = rx.try_recv() {
            saw_reload |= matches!(event, CoreEvent::SessionRewound { .. });
        }
        assert!(saw_reload);
        assert_eq!(fs::read(workspace.join("note.txt")).unwrap(), b"original");
        assert!(!workspace.join("new.txt").exists());
        drop(handle);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn conversation_rewind_works_without_a_workspace_runtime() {
        let (root, store, session, checkpoint, _blobs) = fixture();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (actor, handle) = SessionActor::new(
            store,
            &session,
            Arc::new(UntilCancelled),
            Arc::new(Recorder(tx)),
        )
        .unwrap();
        let task = tokio::spawn(actor.run());
        handle
            .rewind(checkpoint, RewindScope::Conversation)
            .await
            .unwrap();
        assert!(matches!(
            next_event(&mut rx).await,
            CoreEvent::SessionUpdated {
                status: tether_types::SessionStatus::Idle,
                ..
            }
        ));
        assert!(matches!(
            next_event(&mut rx).await,
            CoreEvent::SessionRewound { .. }
        ));
        assert_eq!(
            fs::read(root.path().join("workspace/note.txt")).unwrap(),
            b"latest"
        );
        assert_eq!(
            Store::open(&root.path().join("store.db"))
                .unwrap()
                .load_session(&session)
                .unwrap()
                .unwrap()
                .status,
            tether_types::SessionStatus::Idle
        );
        drop(handle);
        task.await.unwrap();
    }
}
