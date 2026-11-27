use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tether_engine::{SessionActor, SessionActorOptions, ToolRuntime};
use tether_skills::{DiscoverInput, SkillsCatalog, skills_index};
use tether_store::{BlobStore, SessionRecord, Store, project_dir};
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
                    files_root: &self.files_root,
                    database_path: Path::new(&self.database_path),
                    approver: Arc::clone(&self.broker),
                    vault: Arc::clone(&self.vault),
                    mcp_manager: Arc::clone(&self.mcp_manager),
                },
            )?;
            let project = store
                .load_project(&session.project_id)
                .map_err(|_| SessionRuntimeError::Unavailable)?
                .ok_or(SessionRuntimeError::Unavailable)?;
            let catalog = skill_catalog(
                &store,
                Some(tools.guard.root()),
                &self.files_root,
                &session.project_id,
            )?;
            let sources = self.prompt_sources(
                &project.name,
                Some(tools.guard.root().to_owned()),
                Some(skills_index(&catalog, 6_000)),
            );
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
            let catalog = skill_catalog(&store, None, &self.files_root, &session.project_id)?;
            let sources =
                self.prompt_sources(&project.name, None, Some(skills_index(&catalog, 6_000)));
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
    /// Private application files root containing skills and memory.
    pub(super) files_root: &'a Path,
    /// Path of the SQLite database, opened again for the session tools.
    pub(super) database_path: &'a Path,
    /// Approval broker shared with the session.
    pub(super) approver: Arc<ApprovalBroker>,
    /// Secret vault used by the git tools.
    pub(super) vault: Arc<crate::SecretVault>,
    /// Shared MCP manager for this process runtime.
    pub(super) mcp_manager: Arc<tether_engine::McpManager>,
}

#[expect(
    clippy::too_many_lines,
    reason = "the session tool list is built in one place so its dispatch order stays visible"
)]
pub(super) fn session_tool_runtime(
    store: &Store,
    session: &SessionRecord,
    wiring: ToolWiring<'_>,
) -> Result<ToolRuntime, SessionRuntimeError> {
    let root = wiring.root;
    let project = store
        .load_project(&session.project_id)
        .map_err(|_| SessionRuntimeError::Unavailable)?
        .ok_or(SessionRuntimeError::Unavailable)?;
    let project_root = project_dir(root, &project);
    let guard =
        PathGuard::new(project_root.clone()).map_err(|_| SessionRuntimeError::Unavailable)?;
    let blob_root = root
        .parent()
        .ok_or(SessionRuntimeError::Unavailable)?
        .join("blobs");
    let blobs = BlobStore::open(blob_root).map_err(|_| SessionRuntimeError::Unavailable)?;
    let session_store =
        Store::open(wiring.database_path).map_err(|_| SessionRuntimeError::Unavailable)?;
    let knowledge_store =
        Store::open(wiring.database_path).map_err(|_| SessionRuntimeError::Unavailable)?;
    let catalog = skill_catalog(
        store,
        Some(&project_root),
        wiring.files_root,
        &session.project_id,
    )?;
    let git_handler = git_tool_handler(store, session, &wiring)?;
    let mut handlers: Vec<Arc<dyn tether_engine::ToolHandler>> = vec![
        Arc::new(
            crate::executors::session_run_tools(
                wiring.database_path,
                &project_root,
                &session.project_id,
            )
            .map_err(|_| SessionRuntimeError::Unavailable)?,
        ),
        Arc::new(tether_engine::SessionTools::new(
            wiring.approver.questions(),
            Arc::new(Mutex::new(session_store)),
        )),
    ];
    handlers.extend(git_handler);
    handlers.extend(ci_tool_handler(store, session, &project_root, &wiring)?);
    handlers.push(Arc::new(tether_engine::McpToolsHandler::new(Arc::clone(
        &wiring.mcp_manager,
    ))));
    handlers.push(Arc::new(
        tether_engine::WebTools::new().map_err(|_| SessionRuntimeError::Unavailable)?,
    ));
    handlers.push(Arc::new(tether_engine::KnowledgeTools::new(
        catalog,
        wiring.files_root.join("skills"),
        wiring.files_root.join("memory"),
        wiring.files_root.join("memory").join("vault"),
        Arc::new(Mutex::new(knowledge_store)),
    )));
    let approver_dyn: Arc<dyn tether_engine::Approver> = wiring.approver;
    Ok(ToolRuntime {
        guard,
        blobs,
        approver: approver_dyn,
        mode: Arc::new(Mutex::new(session.mode)),
        handlers,
    })
}

fn skill_catalog(
    store: &Store,
    project_root: Option<&Path>,
    files_root: &Path,
    project_id: &tether_types::ProjectId,
) -> Result<SkillsCatalog, SessionRuntimeError> {
    let trusted = store
        .is_project_trusted(project_id)
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    let mut catalog = SkillsCatalog::discover(DiscoverInput {
        project_root,
        project_trusted: trusted,
        user_dir: &files_root.join("skills"),
        bundled: &[],
    });
    for name in store
        .disabled_skills()
        .map_err(|_| SessionRuntimeError::Unavailable)?
    {
        catalog.set_disabled(&name, true);
    }
    Ok(catalog)
}

/// Builds the git tool handler for every project workspace.
fn git_tool_handler(
    store: &Store,
    session: &SessionRecord,
    wiring: &ToolWiring<'_>,
) -> Result<Option<Arc<dyn tether_engine::ToolHandler>>, SessionRuntimeError> {
    let remote = store
        .project_remote(&session.project_id)
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    let git_runtime = crate::GitRuntime::new(
        wiring.database_path.to_string_lossy().as_ref(),
        wiring.root.to_string_lossy().as_ref(),
        Arc::clone(&wiring.vault),
    )
    .map_err(|_| SessionRuntimeError::Unavailable)?;
    let github_owner = store
        .git_identity()
        .map_err(|_| SessionRuntimeError::Unavailable)?
        .and_then(|identity| identity.login);
    Ok(Some(crate::git::GitRuntime::tool_handler(
        &git_runtime,
        session.project_id.clone(),
        remote.as_ref().and_then(|value| value.remote_url.clone()),
        remote.and_then(|value| value.default_branch),
        github_owner,
    )))
}

fn ci_tool_handler(
    store: &Store,
    session: &SessionRecord,
    project_root: &Path,
    wiring: &ToolWiring<'_>,
) -> Result<Option<Arc<dyn tether_engine::ToolHandler>>, SessionRuntimeError> {
    let remote = store
        .project_remote(&session.project_id)
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    let Some(remote_url) = remote
        .as_ref()
        .and_then(|value| value.remote_url.as_deref())
    else {
        return Ok(None);
    };
    if tether_vcs::parse_github_url(remote_url).is_none()
        || tether_vcs::Repo::open(project_root).is_err()
    {
        return Ok(None);
    }
    let runtime = crate::CiRuntime::new(
        wiring.database_path.to_string_lossy().as_ref(),
        wiring.root.to_string_lossy().as_ref(),
        Arc::clone(&wiring.vault),
    )
    .map_err(|_| SessionRuntimeError::Unavailable)?;
    let branch = tether_vcs::Repo::open(project_root)
        .ok()
        .and_then(|repo| repo.current_branch().ok().flatten())
        .or_else(|| remote.and_then(|value| value.default_branch))
        .unwrap_or_else(|| "main".into());
    Ok(Some(crate::ci::CiRuntime::tool_handler(
        runtime,
        session.project_id.clone(),
        branch,
    )))
}
