use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tether_engine::{SessionActor, SessionActorOptions, ToolRuntime};
use tether_store::{BlobStore, SessionRecord, Store};
use tether_types::SessionId;
use tether_workspace::PathGuard;

use super::{
    ApprovalBroker, SessionRunner, SessionRuntime, SessionRuntimeError, provider_key,
    session_provider_factory,
};

impl SessionRuntime {
    #[expect(
        clippy::too_many_lines,
        reason = "opening a runner validates credentials and wires prompt sources"
    )]
    pub(super) fn open(
        &self,
        session_id: &str,
        workspaces_root: Option<&Path>,
    ) -> Result<Arc<SessionRunner>, SessionRuntimeError> {
        let id = SessionId::new(session_id);
        let mut runners = self
            .runners
            .lock()
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        if let Some(existing) = runners.get(&id) {
            return Ok(Arc::clone(existing));
        }
        let store = self.store()?;
        let session = store
            .load_session(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .ok_or(SessionRuntimeError::NotFound)?;
        provider_key(&self.vault, session.provider)?;
        let provider_factory = session_provider_factory(Arc::clone(&self.vault), session.provider);
        let sink = self.session_sink();
        let (actor, handle) = if let Some(root) = workspaces_root {
            let tools = session_tool_runtime(
                &store,
                &session,
                ToolWiring {
                    root,
                    database_path: Path::new(&self.database_path),
                    approver: Arc::clone(&self.broker),
                    vault: Arc::clone(&self.vault),
                },
            )?;
            let project = store
                .load_project(&session.project_id)
                .map_err(|_| SessionRuntimeError::Unavailable)?
                .ok_or(SessionRuntimeError::Unavailable)?;
            let sources = self.prompt_sources(&project.name, Some(tools.guard.root().to_owned()));
            self.project_roots
                .lock()
                .map_err(|_| SessionRuntimeError::Unavailable)?
                .insert(id.clone(), tools.guard.root().to_owned());
            SessionActor::new_with_options(
                store,
                &id,
                provider_factory,
                sink,
                SessionActorOptions {
                    tools: Some(tools),
                    prompt_sources: sources,
                    network: Some(Arc::clone(&self.network)),
                },
            )
        } else {
            let project = store
                .load_project(&session.project_id)
                .map_err(|_| SessionRuntimeError::Unavailable)?
                .ok_or(SessionRuntimeError::Unavailable)?;
            let sources = self.prompt_sources(&project.name, None);
            SessionActor::new_with_options(
                store,
                &id,
                provider_factory,
                sink,
                SessionActorOptions {
                    tools: None,
                    prompt_sources: sources,
                    network: Some(Arc::clone(&self.network)),
                },
            )
        }
        .map_err(|_| SessionRuntimeError::Unavailable)?;
        self.runtime.spawn(actor.run());
        let runner = Arc::new(SessionRunner {
            handle,
            runtime: Arc::clone(&self.runtime),
            valid: AtomicBool::new(true),
        });
        runners.insert(id, Arc::clone(&runner));
        Ok(runner)
    }
}

/// Paths and shared services that a session's tool runtime is built from.
pub(super) struct ToolWiring<'a> {
    /// Workspaces root folder that holds the project folders.
    pub(super) root: &'a Path,
    /// Path of the SQLite database, opened again for the session tools.
    pub(super) database_path: &'a Path,
    /// Approval broker shared with the session.
    pub(super) approver: Arc<ApprovalBroker>,
    /// Secret vault used by the git tools.
    pub(super) vault: Arc<crate::SecretVault>,
}

pub(super) fn session_tool_runtime(
    store: &Store,
    session: &SessionRecord,
    wiring: ToolWiring<'_>,
) -> Result<ToolRuntime, SessionRuntimeError> {
    let root = wiring.root;
    let root_guard = PathGuard::new(root).map_err(|_| SessionRuntimeError::Unavailable)?;
    let project_path = store
        .project_path(&session.project_id)
        .map_err(|_| SessionRuntimeError::Unavailable)?
        .ok_or(SessionRuntimeError::Unavailable)?;
    let project_root = root_guard
        .resolve(&project_path)
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    let guard =
        PathGuard::new(project_root.clone()).map_err(|_| SessionRuntimeError::Unavailable)?;
    let blob_root = root
        .parent()
        .ok_or(SessionRuntimeError::Unavailable)?
        .join("blobs");
    let blobs = BlobStore::open(blob_root).map_err(|_| SessionRuntimeError::Unavailable)?;
    let session_store =
        Store::open(wiring.database_path).map_err(|_| SessionRuntimeError::Unavailable)?;
    let git_handler = git_tool_handler(store, session, &project_root, &wiring)?;
    let mut handlers: Vec<Arc<dyn tether_engine::ToolHandler>> = vec![
        Arc::new(
            tether_engine::executors::RunTools::new(
                root.parent()
                    .ok_or(SessionRuntimeError::Unavailable)?
                    .join("runtimes"),
            )
            .map_err(|_| SessionRuntimeError::Unavailable)?,
        ),
        Arc::new(tether_engine::SessionTools::new(
            wiring.approver.questions(),
            Arc::new(Mutex::new(session_store)),
        )),
    ];
    handlers.extend(git_handler);
    let approver_dyn: Arc<dyn tether_engine::Approver> = wiring.approver;
    Ok(ToolRuntime {
        guard,
        blobs,
        approver: approver_dyn,
        mode: Arc::new(Mutex::new(session.mode)),
        handlers,
    })
}

/// Builds the git tool handler when the project has a repository or a remote URL.
fn git_tool_handler(
    store: &Store,
    session: &SessionRecord,
    project_root: &Path,
    wiring: &ToolWiring<'_>,
) -> Result<Option<Arc<dyn tether_engine::ToolHandler>>, SessionRuntimeError> {
    let remote = store
        .project_remote(&session.project_id)
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    let has_git = tether_vcs::Repo::open(project_root).is_ok();
    let has_remote_url = remote
        .as_ref()
        .and_then(|value| value.remote_url.as_ref())
        .is_some();
    if !has_git && !has_remote_url {
        return Ok(None);
    }
    let git_runtime = crate::GitRuntime::new(
        wiring.database_path.to_string_lossy().as_ref(),
        wiring.root.to_string_lossy().as_ref(),
        Arc::clone(&wiring.vault),
    )
    .map_err(|_| SessionRuntimeError::Unavailable)?;
    Ok(Some(crate::git::GitRuntime::tool_handler(
        git_runtime,
        remote.as_ref().and_then(|value| value.remote_url.clone()),
        remote.and_then(|value| value.default_branch),
    )))
}
