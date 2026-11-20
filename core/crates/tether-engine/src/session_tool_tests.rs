use std::fs;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use tether_providers::{
    BoxFuture, ChatProvider, ChatRequest, ProviderError, StopReason, StreamEvent,
};
use tether_store::{BlobStore, Store};
use tether_types::{
    CoreEvent, MessagePart, ModelRef, PermissionMode, ProjectId, Provider, Role, ToolCallId,
};
use tether_workspace::PathGuard;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{EventSink, ProviderFactory, SessionActor, ToolRuntime};

struct ReadThenFinish(AtomicUsize);

impl ChatProvider for ReadThenFinish {
    #[expect(
        clippy::excessive_nesting,
        reason = "the scripted provider sends two distinct responses"
    )]
    fn stream_chat<'a>(
        &'a self,
        request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            let step = self.0.fetch_add(1, Ordering::SeqCst);
            assert_eq!(request.tools.len(), 9);
            if step == 0 {
                let id = ToolCallId::new("read-1");
                events
                    .send(StreamEvent::ToolCallStarted {
                        id: id.clone(),
                        name: "read_file".into(),
                    })
                    .await
                    .unwrap();
                events
                    .send(StreamEvent::ToolCallArgsDelta {
                        id: id.clone(),
                        json_fragment: r#"{"path":"note.txt"}"#.into(),
                    })
                    .await
                    .unwrap();
                events
                    .send(StreamEvent::ToolCallFinished { id })
                    .await
                    .unwrap();
                events
                    .send(StreamEvent::Finished {
                        stop_reason: StopReason::ToolUse,
                    })
                    .await
                    .unwrap();
            } else {
                assert_eq!(step, 1);
                assert_eq!(request.messages.len(), 3);
                events
                    .send(StreamEvent::TextDelta {
                        text: "Done".into(),
                    })
                    .await
                    .unwrap();
                events
                    .send(StreamEvent::Finished {
                        stop_reason: StopReason::EndTurn,
                    })
                    .await
                    .unwrap();
            }
            Ok(())
        })
    }
}

struct Recorder(mpsc::UnboundedSender<CoreEvent>);

impl EventSink for Recorder {
    fn on_event(&self, event: CoreEvent) {
        self.0.send(event).unwrap();
    }
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    clippy::excessive_nesting,
    reason = "the actor fixture sets up storage and checks the complete exchange"
)]
async fn session_actor_runs_read_tool_and_reopens_saved_exchange() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::write(workspace.join("note.txt"), "hello").unwrap();
    let database = root.path().join("store.db");
    drop(Store::open(&database).unwrap());
    rusqlite::Connection::open(&database).unwrap().execute(
        "INSERT INTO projects (id, name, path, created_at) VALUES ('project', 'fixture', 'fixture', 1)",
        [],
    ).unwrap();
    let mut store = Store::open(&database).unwrap();
    let session = store
        .create_session(
            &ProjectId::new("project"),
            &ModelRef {
                provider: Provider::OpenAi,
                model_id: "fixture".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    let id = session.id;
    let provider: Arc<dyn ChatProvider> = Arc::new(ReadThenFinish(AtomicUsize::new(0)));
    let factory: ProviderFactory = Arc::new(move || Ok(Arc::clone(&provider)));
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (actor, handle) = SessionActor::new_with_tool_runtime(
        store,
        &id,
        factory,
        Arc::new(Recorder(tx)),
        ToolRuntime {
            guard: PathGuard::new(&workspace).unwrap(),
            blobs: BlobStore::open(root.path().join("blobs")).unwrap(),
            approve: Arc::new(|_, _| false),
        },
    )
    .unwrap();
    let task = tokio::spawn(actor.run());
    handle.send_message("Read the note".into()).await.unwrap();
    loop {
        let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        if matches!(event, CoreEvent::TurnFinished { .. }) {
            break;
        }
    }
    drop(handle);
    task.await.unwrap();
    let messages = Store::open(&database).unwrap().messages(&id).unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[1].role, Role::Assistant);
    assert!(
        matches!(&messages[2].parts[0], MessagePart::ToolResult { text, .. } if text.contains("hello"))
    );
    assert_eq!(
        messages[3].parts,
        vec![MessagePart::Text {
            text: "Done".into()
        }]
    );
}
