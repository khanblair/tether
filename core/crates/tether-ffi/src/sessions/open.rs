use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tether_engine::{
    ProviderFactory, SessionActor, SessionActorOptions, SessionHandle, ToolRuntime,
};
use tether_store::{BlobStore, SessionRecord, Store};
use tether_types::{Provider, SessionId};
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
                root,
                Arc::clone(&self.broker),
                Path::new(&self.database_path),
                Arc::clone(&self.vault),
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

fn session_tool_runtime(
    store: &Store,
    session: &SessionRecord,
    root: &Path,
    approver: Arc<ApprovalBroker>,
    database_path: &Path,
    vault: Arc<crate::SecretVault>,
) -> Result<ToolRuntime, SessionRuntimeError> {
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
    let session_store = Store::open(database_path).map_err(|_| SessionRuntimeError::Unavailable)?;
    let remote = store
        .project_remote(&session.project_id)
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    let git_root = project_root.clone();
    let has_git = tether_vcs::Repo::open(&git_root).is_ok();
    let git_handler = if has_git
        || remote
            .as_ref()
            .and_then(|value| value.remote_url.as_ref())
            .is_some()
    {
        let git_runtime = crate::GitRuntime::new(
            database_path.to_string_lossy().as_ref(),
            root.to_string_lossy().as_ref(),
            vault,
        )
        .map_err(|_| SessionRuntimeError::Unavailable)?;
        Some(crate::git::GitRuntime::tool_handler(
            git_runtime,
            remote.as_ref().and_then(|value| value.remote_url.clone()),
            remote.and_then(|value| value.default_branch),
        ))
    } else {
        None
    };
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
            approver.questions(),
            Arc::new(Mutex::new(session_store)),
        )),
    ];
    if let Some(git_handler) = git_handler {
        handlers.push(git_handler);
    }
    Ok(ToolRuntime {
        guard,
        blobs,
        approver: Arc::clone(&approver),
        mode: Arc::new(Mutex::new(session.mode)),
        handlers,
    })
}
