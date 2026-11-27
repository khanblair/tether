use super::open::{ToolWiring, session_tool_runtime};
use super::ux::map_rewind_error;
use super::*;
use crate::SecretsPlatform;
use crate::SessionStatusView;
use tether_engine::{ApprovalAnswer, ApprovalRequest, Approver, ToolLoop};
use tether_store::BlobStore;
use tether_types::{ModelRef, SessionStatus};

#[test]
fn session_list_orders_activity_and_rejects_unknown_project() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let mut store = Store::open(&db).unwrap();
    let project = store.create_project("Project", 1).unwrap();
    let old = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "saved".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap();
    let new = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "saved".into(),
            },
            PermissionMode::Ask,
            20,
        )
        .unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    let listed = runtime.list_sessions(project.id.as_str().into()).unwrap();
    assert_eq!(
        listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        [new.id.as_str(), old.id.as_str()]
    );
    assert!(matches!(
        runtime.list_sessions("missing".into()),
        Err(SessionRuntimeError::NotFound)
    ));
}

#[test]
fn global_instructions_round_trip_with_cap_and_removal() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    Store::open(&db).unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();

    runtime
        .set_global_instructions(Some("Use the project checks.".into()))
        .unwrap();
    assert_eq!(
        runtime.global_instructions().unwrap(),
        Some("Use the project checks.".into())
    );
    assert!(matches!(
        runtime.set_global_instructions(Some("x".repeat(32 * 1024 + 1))),
        Err(SessionRuntimeError::InstructionsTooLong)
    ));
    runtime.set_global_instructions(None).unwrap();
    assert_eq!(runtime.global_instructions().unwrap(), None);
}

#[test]
fn prompt_sections_are_ordered_and_have_token_counts() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    let sections = runtime
        .session_prompt_sections(session_id.as_str())
        .unwrap();

    assert_eq!(
        sections.first().map(|section| section.name.as_str()),
        Some("Base")
    );
    assert_eq!(
        sections.last().map(|section| section.name.as_str()),
        Some("Date")
    );
    assert!(
        sections
            .iter()
            .all(|section| { section.approx_tokens == section.chars.saturating_add(3) / 4 })
    );
}

#[test]
fn default_model_reads_first_saved_model_or_none() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let mut store = Store::open(&db).unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    assert_eq!(
        runtime.default_model(ProviderView::Anthropic).unwrap(),
        None
    );
    let model = |id: &str| tether_types::ModelInfo {
        provider: Provider::Anthropic,
        id: id.into(),
        display_name: id.into(),
        context_window: None,
        max_output_tokens: None,
        price: None,
        caps: tether_types::ModelCaps::UNKNOWN,
    };
    store
        .replace_models(Provider::Anthropic, &[model("first"), model("second")], 1)
        .unwrap();
    assert_eq!(
        runtime.default_model(ProviderView::Anthropic).unwrap(),
        Some("first".into())
    );
}

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

struct EngineNoopSink;

impl tether_engine::EventSink for EngineNoopSink {
    fn on_event(&self, _event: tether_types::CoreEvent) {}
}

fn seeded_session(db: &Path) -> SessionId {
    let mut store = Store::open(db).unwrap();
    let project = ProjectId::new("01TESTPROJECT00000000000000");
    rusqlite::Connection::open(db)
        .unwrap()
        .execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, 'Test', 'test', 1)",
            [project.as_str()],
        )
        .unwrap();
    let session = store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "test-model".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    store.begin_turn(&session.id, "Hi", 2).unwrap();
    session.id
}

#[test]
fn reopening_recovers_an_active_turn_and_reuses_its_actor() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);
    assert_eq!(
        recover_interrupted_sessions(db.to_str().unwrap()).unwrap(),
        1
    );

    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime =
        SessionRuntime::new(db.to_str().unwrap(), Arc::clone(&vault), Arc::new(NoopSink)).unwrap();
    let view = runtime.load_session(session_id.as_str()).unwrap();
    assert_eq!(view.status, SessionStatusView::Interrupted);
    assert_eq!(runtime.transcript(session_id.as_str()).unwrap().len(), 1);
    assert!(matches!(
        runtime.open_session(session_id.as_str()),
        Err(SessionRuntimeError::MissingKey)
    ));
    vault.put("provider-anthropic", "test-key").unwrap();
    let first = runtime.open_session(session_id.as_str()).unwrap();
    let again = runtime.open_session(session_id.as_str()).unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    assert_eq!(
        Store::open(&db)
            .unwrap()
            .load_session(&session_id)
            .unwrap()
            .unwrap()
            .status,
        SessionStatus::Interrupted
    );
}

#[test]
fn workspace_session_requires_a_saved_project_folder() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    vault.put("provider-anthropic", "test-key").unwrap();
    let runtime =
        SessionRuntime::new(db.to_str().unwrap(), Arc::clone(&vault), Arc::new(NoopSink)).unwrap();
    let root = dir.path().join("workspaces");
    std::fs::create_dir(&root).unwrap();
    assert!(matches!(
        runtime.open_session_with_workspace(session_id.as_str(), root.to_str().unwrap()),
        Err(SessionRuntimeError::Unavailable)
    ));
    std::fs::create_dir(root.join("test")).unwrap();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(
            "UPDATE sessions SET mode = 'accept_edits' WHERE id = ?1",
            [session_id.as_str()],
        )
        .unwrap();
    let store = Store::open(&db).unwrap();
    let saved = store.load_session(&session_id).unwrap().unwrap();
    let mcp_manager = crate::mcp::manager_from_store(&store, &vault).unwrap();
    let tools = session_tool_runtime(
        &store,
        &saved,
        ToolWiring {
            root: &root,
            files_root: &root,
            database_path: &db,
            approver: Arc::new(ApprovalBroker::new()),
            vault,
            mcp_manager,
            sink: Arc::new(EngineNoopSink),
        },
    )
    .unwrap();
    assert!(!tools.0.approver.interactive());
    assert!(
        runtime
            .open_session_with_workspace(session_id.as_str(), root.to_str().unwrap())
            .is_ok()
    );
}

#[test]
fn plain_project_sessions_advertise_git_tools_for_initialization() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);
    let root = dir.path().join("workspaces");
    std::fs::create_dir_all(root.join("test")).unwrap();
    let store = Store::open(&db).unwrap();
    let session = store.load_session(&session_id).unwrap().unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let mcp_manager = crate::mcp::manager_from_store(&store, &vault).unwrap();
    let tools = session_tool_runtime(
        &store,
        &session,
        ToolWiring {
            root: &root,
            files_root: &root,
            database_path: &db,
            approver: Arc::new(ApprovalBroker::new()),
            vault,
            mcp_manager,
            sink: Arc::new(EngineNoopSink),
        },
    )
    .unwrap();
    let registry = ToolLoop::with_handlers(tools.0.guard, tools.0.handlers).unwrap();
    let names: Vec<_> = registry.specs().into_iter().map(|spec| spec.name).collect();
    assert!(names.contains(&"git_init".to_owned()));
    assert!(names.contains(&"github_repo_create".to_owned()));
}

#[test]
fn checkpoint_list_exposes_saved_paths_and_message_anchor() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);
    let mut store = Store::open(&db).unwrap();
    let anchor = store.messages(&session_id).unwrap()[0].id.clone();
    let blobs = BlobStore::open(dir.path().join("blobs")).unwrap();
    let checkpoint = store
        .save_checkpoint(
            &session_id,
            Some(&anchor),
            &blobs,
            &[tether_store::SnapshotInput {
                path: "note.txt".into(),
                content: None,
            }],
            4,
        )
        .unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    let listed = runtime.list_checkpoints(session_id.as_str()).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, checkpoint.id);
    assert_eq!(listed[0].message_id.as_deref(), Some(anchor.as_str()));
    assert_eq!(listed[0].paths, vec!["note.txt"]);
}

#[test]
fn provider_factory_reads_the_current_key_after_removal() {
    let dir = tempfile::tempdir().unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    vault.put("provider-anthropic", "old-key").unwrap();
    let factory = session_provider_factory(Arc::clone(&vault), Provider::Anthropic);
    assert!(factory().is_ok());
    vault.delete("provider-anthropic").unwrap();
    assert!(matches!(
        factory(),
        Err(tether_types::CoreError::Auth { .. })
    ));
    vault.put("provider-anthropic", "new-key").unwrap();
    assert!(factory().is_ok());
}

pub(super) fn ux_runtime() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    Arc<SessionRuntime>,
    SessionId,
    ProjectId,
) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let mut store = Store::open(&db).unwrap();
    let project = store.create_project("Project", 1).unwrap();
    let session = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "first".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    (dir, db, runtime, session.id, project.id)
}

#[test]
fn interactive_approval_answers_once_and_unknown_ids_are_not_found() {
    let (_dir, _db, runtime, session, _project) = ux_runtime();
    runtime.runtime.block_on(async {
        let request = ApprovalRequest {
            session_id: session.clone(),
            tool_call_id: tether_types::ToolCallId::new("call-1"),
            tool: "write_file".into(),
            target: "note.txt".into(),
        };
        let denied = runtime
            .broker
            .decide(request.clone(), tokio_util::sync::CancellationToken::new())
            .await;
        assert_eq!(
            denied,
            ApprovalAnswer::Deny {
                reason: Some("approvals need a newer app version".into())
            }
        );
        runtime.set_interactive_approvals(true);
        let broker = Arc::clone(&runtime.broker);
        let waiting = tokio::spawn(async move {
            broker
                .decide(request, tokio_util::sync::CancellationToken::new())
                .await
        });
        tokio::task::yield_now().await;
        assert_eq!(
            runtime.pending_approvals(session.as_str()).unwrap().len(),
            1
        );
        runtime
            .approve_from_notification(session.as_str(), "call-1")
            .unwrap();
        assert_eq!(
            waiting.await.unwrap(),
            ApprovalAnswer::AllowFromNotification { always: false }
        );
        assert!(matches!(
            runtime.deny_from_notification(session.as_str(), "call-1"),
            Err(SessionRuntimeError::NotFound)
        ));
    });
}

#[test]
fn model_picker_reads_saved_order_fields_and_badges() {
    let (_dir, db, runtime, _session, _project) = ux_runtime();
    let mut store = Store::open(&db).unwrap();
    let model = |id: &str, price: Option<tether_types::ModelPrice>| tether_types::ModelInfo {
        provider: Provider::Anthropic,
        id: id.into(),
        display_name: format!("Name {id}"),
        context_window: Some(200_000),
        max_output_tokens: Some(8_192),
        price,
        caps: tether_types::ModelCaps {
            tools: tether_types::Support::Yes,
            images: tether_types::Support::No,
            reasoning: tether_types::Support::Unknown,
        },
    };
    store
        .replace_models(
            Provider::Anthropic,
            &[
                model("second", None),
                model(
                    "first",
                    Some(tether_types::ModelPrice {
                        input_micros: 3_000_000,
                        output_micros: 15_000_000,
                        cache_read_micros: None,
                        cache_write_micros: None,
                        source: tether_types::PriceSource::Bundled,
                    }),
                ),
            ],
            1,
        )
        .unwrap();
    let listed = runtime.list_models(ProviderView::Anthropic).unwrap();
    assert_eq!(
        listed.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        ["second", "first"]
    );
    assert_eq!(listed[0].display_name, "Name second");
    assert_eq!(listed[0].context_window, Some(200_000));
    assert_eq!(listed[0].max_output_tokens, Some(8_192));
    assert_eq!(listed[0].input_price_micros_per_million, None);
    assert_eq!(listed[1].input_price_micros_per_million, Some(3_000_000));
    assert_eq!(listed[1].output_price_micros_per_million, Some(15_000_000));
    assert_eq!(listed[0].tools, crate::SupportView::Yes);
    assert_eq!(listed[0].images, crate::SupportView::No);
    assert_eq!(listed[0].reasoning, crate::SupportView::Unknown);
}

#[test]
fn session_settings_change_and_busy_or_missing_ids_report_exact_errors() {
    let (_dir, db, runtime, session, _project) = ux_runtime();
    runtime
        .set_session_model(session.as_str(), ProviderView::Gemini, "second")
        .unwrap();
    runtime
        .set_session_mode(session.as_str(), crate::PermissionModeView::Auto)
        .unwrap();
    let saved = runtime.load_session(session.as_str()).unwrap();
    assert_eq!(saved.provider, ProviderView::Gemini);
    assert_eq!(saved.model_id, "second");
    assert_eq!(saved.mode, crate::PermissionModeView::Auto);
    assert!(matches!(
        runtime.set_session_model(session.as_str(), ProviderView::Gemini, " "),
        Err(SessionRuntimeError::EmptyInput)
    ));
    assert!(matches!(
        runtime.set_session_model("missing", ProviderView::Gemini, "id"),
        Err(SessionRuntimeError::NotFound)
    ));
    assert!(matches!(
        runtime.set_session_mode("missing", crate::PermissionModeView::Plan),
        Err(SessionRuntimeError::NotFound)
    ));
    Store::open(&db)
        .unwrap()
        .set_session_status(&session, SessionStatus::Streaming, 50)
        .unwrap();
    assert!(matches!(
        runtime.set_session_model(session.as_str(), ProviderView::Gemini, "third"),
        Err(SessionRuntimeError::Busy)
    ));
    assert!(matches!(
        runtime.set_session_mode(session.as_str(), crate::PermissionModeView::Plan),
        Err(SessionRuntimeError::Busy)
    ));
}

#[test]
fn changing_settings_opens_a_runner_with_new_saved_settings() {
    let (_dir, _db, runtime, session, _project) = ux_runtime();
    runtime.vault.put("provider-anthropic", "test-key").unwrap();
    runtime.vault.put("provider-gemini", "test-key").unwrap();
    let old = runtime.open_session(session.as_str()).unwrap();
    runtime
        .set_session_model(session.as_str(), ProviderView::Gemini, "second")
        .unwrap();
    let current = runtime.open_session(session.as_str()).unwrap();
    assert!(!Arc::ptr_eq(&old, &current));
    assert!(matches!(
        old.send_message("stale".into()),
        Err(SessionRuntimeError::Unavailable)
    ));
    runtime
        .set_session_mode(session.as_str(), crate::PermissionModeView::Auto)
        .unwrap();
    let after_mode = runtime.open_session(session.as_str()).unwrap();
    assert!(!Arc::ptr_eq(&current, &after_mode));
    assert!(matches!(
        current.send_message("stale".into()),
        Err(SessionRuntimeError::Unavailable)
    ));
}

#[test]
fn rename_is_available_through_runtime() {
    let (_dir, _db, runtime, session, _project) = ux_runtime();
    runtime
        .rename_session(session.as_str(), "  New title  ")
        .unwrap();
    assert_eq!(
        runtime.load_session(session.as_str()).unwrap().title,
        "New title"
    );
    assert!(matches!(
        runtime.rename_session(session.as_str(), "  "),
        Err(SessionRuntimeError::EmptyInput)
    ));
    assert!(matches!(
        runtime.rename_session("missing", "Name"),
        Err(SessionRuntimeError::NotFound)
    ));
}

#[test]
fn usage_is_available_through_runtime() {
    let (_dir, db, runtime, session, _project) = ux_runtime();
    let store = Store::open(&db).unwrap();
    store
        .add_session_usage(
            &session,
            tether_types::UsageSnapshot {
                tokens_in: 12,
                tokens_out: 7,
                cost_micros: 42,
            },
            20,
        )
        .unwrap();
    assert_eq!(
        runtime.session_usage(session.as_str()).unwrap().tokens_in,
        12
    );
    assert_eq!(
        runtime.session_usage(session.as_str()).unwrap().tokens_out,
        7
    );
    assert_eq!(
        runtime.session_usage(session.as_str()).unwrap().cost_micros,
        42
    );
    assert!(matches!(
        runtime.session_usage("missing"),
        Err(SessionRuntimeError::NotFound)
    ));
}

#[test]
fn session_and_transcript_search_are_available_through_runtime() {
    let (_dir, db, runtime, session, project) = ux_runtime();
    runtime
        .rename_session(session.as_str(), "New title")
        .unwrap();
    let mut store = Store::open(&db).unwrap();
    let message = store.begin_turn(&session, "Hidden lantern", 21).unwrap();
    assert_eq!(
        runtime.search_sessions(project.as_str(), "title").unwrap()[0].id,
        session.as_str()
    );
    assert_eq!(
        runtime
            .search_sessions(project.as_str(), "lantern")
            .unwrap()[0]
            .id,
        session.as_str()
    );
    let hit = runtime.search_transcripts("hidden lantern", 100).unwrap();
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].message_id, message.as_str());
    assert_eq!(hit[0].session_id, session.as_str());
    assert_eq!(hit[0].session_title, "New title");
    assert_eq!(hit[0].text, "Hidden lantern");
    assert!(matches!(
        runtime.search_sessions("missing", "title"),
        Err(SessionRuntimeError::NotFound)
    ));
}

#[test]
fn transcript_search_caps_the_requested_limit_at_fifty() {
    let (_dir, db, runtime, session, _project) = ux_runtime();
    let mut store = Store::open(&db).unwrap();
    for index in 0..55 {
        store
            .append_message(
                &session,
                tether_types::Role::User,
                &[tether_types::MessagePart::Text {
                    text: format!("needle {index}"),
                }],
                100 + index,
            )
            .unwrap();
    }
    let hits = runtime.search_transcripts("needle", 500).unwrap();
    assert_eq!(hits.len(), 50);
    assert_eq!(runtime.search_transcripts("needle", 0).unwrap().len(), 0);
}

#[test]
fn ffi_checkpoint_list_hides_rewound_anchors() {
    let (dir, db, runtime, session, _project) = ux_runtime();
    let mut store = Store::open(&db).unwrap();
    let anchor = store.begin_turn(&session, "first", 20).unwrap();
    let blobs = BlobStore::open(dir.path().join("blobs")).unwrap();
    let input = [tether_store::SnapshotInput {
        path: "note.txt".into(),
        content: None,
    }];
    store
        .save_checkpoint(&session, Some(&anchor), &blobs, &input, 21)
        .unwrap();
    let live = store
        .save_checkpoint(&session, None, &blobs, &input, 22)
        .unwrap();
    store.rewind_conversation(&session, &anchor).unwrap();
    let listed = runtime.list_checkpoints(session.as_str()).unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|checkpoint| checkpoint.id.as_str())
            .collect::<Vec<_>>(),
        [live.id.as_str()]
    );
}

#[test]
fn rewind_errors_keep_corrupt_snapshot_and_invalid_path() {
    use std::io;
    use tether_engine::{RewindError, SessionActorError};

    assert!(matches!(
        map_rewind_error(&SessionActorError::Rewind(RewindError::CorruptSnapshot)),
        SessionRuntimeError::CorruptSnapshot
    ));
    assert!(matches!(
        map_rewind_error(&SessionActorError::Rewind(RewindError::InvalidPath)),
        SessionRuntimeError::InvalidPath
    ));
    assert!(matches!(
        map_rewind_error(&SessionActorError::Rewind(RewindError::Io(
            io::Error::other("disk")
        ))),
        SessionRuntimeError::Unavailable
    ));
}

fn set_status(db: &Path, id: &SessionId, status: &str) {
    rusqlite::Connection::open(db)
        .unwrap()
        .execute(
            "UPDATE sessions SET status = ?1 WHERE id = ?2",
            [status, id.as_str()],
        )
        .unwrap();
}

fn saved_status(db: &Path, id: &SessionId) -> SessionStatus {
    Store::open(db)
        .unwrap()
        .load_session(id)
        .unwrap()
        .unwrap()
        .status
}

#[test]
fn building_runtimes_does_not_change_a_streaming_turn() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);
    set_status(&db, &session_id, "streaming");
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );

    let _first =
        SessionRuntime::new(db.to_str().unwrap(), Arc::clone(&vault), Arc::new(NoopSink)).unwrap();
    let _second =
        SessionRuntime::new(db.to_str().unwrap(), Arc::clone(&vault), Arc::new(NoopSink)).unwrap();

    assert_eq!(saved_status(&db, &session_id), SessionStatus::Streaming);
}

#[test]
fn recovery_reports_the_changed_count_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);
    set_status(&db, &session_id, "streaming");

    assert_eq!(
        recover_interrupted_sessions(db.to_str().unwrap()).unwrap(),
        1
    );
    assert_eq!(saved_status(&db, &session_id), SessionStatus::Interrupted);
    assert_eq!(
        recover_interrupted_sessions(db.to_str().unwrap()).unwrap(),
        0
    );
}
