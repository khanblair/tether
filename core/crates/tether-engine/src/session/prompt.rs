use std::sync::{Arc, Mutex};

use tether_store::Store;
use tether_types::SessionId;
use tokio::sync::mpsc;

use super::{
    ProviderFactory, SessionActor, SessionActorError, SessionActorOptions, SessionHandle,
    ToolRuntime,
};
use crate::prompt::{self, PromptSources};

impl SessionActor {
    /// Loads a session with credentials resolved anew for each turn.
    ///
    /// # Errors
    /// Returns an error when the id is absent or the store cannot read it.
    pub fn new_with_provider_factory(
        store: Store,
        id: &SessionId,
        provider_for_turn: ProviderFactory,
        sink: Arc<dyn crate::EventSink>,
    ) -> Result<(Self, SessionHandle), SessionActorError> {
        Self::new_with_provider_factory_and_sources(
            store,
            id,
            provider_for_turn,
            sink,
            PromptSources::default(),
        )
    }

    /// Loads a session with prompt sources that are read at open or at each turn.
    ///
    /// # Errors
    /// Returns an error when the id, store, or frozen prompt snapshot is unavailable.
    pub fn new_with_provider_factory_and_sources(
        store: Store,
        id: &SessionId,
        provider_for_turn: ProviderFactory,
        sink: Arc<dyn crate::EventSink>,
        prompt_sources: PromptSources,
    ) -> Result<(Self, SessionHandle), SessionActorError> {
        Self::new_with_options(
            store,
            id,
            provider_for_turn,
            sink,
            SessionActorOptions {
                tools: None,
                prompt_sources,
                network: None,
            },
        )
    }

    /// Loads an actor with an optional tool runtime and prompt sources.
    ///
    /// # Errors
    /// Returns an error when the id, store, or frozen prompt snapshot is unavailable.
    pub fn new_with_options(
        store: Store,
        id: &SessionId,
        provider_for_turn: ProviderFactory,
        sink: Arc<dyn crate::EventSink>,
        options: SessionActorOptions,
    ) -> Result<(Self, SessionHandle), SessionActorError> {
        let session = store.load_session(id)?.ok_or(SessionActorError::NotFound)?;
        let project_name = store.load_project(&session.project_id)?.map_or_else(
            || session.project_id.as_str().to_owned(),
            |project| project.name,
        );
        let prompt_snapshot = load_snapshot(&store, id, &options.prompt_sources)?;
        let (tx, rx) = mpsc::channel(8);
        let active = Arc::new(Mutex::new(None));
        let budget = crate::budget::BudgetGate::new();
        let bypass_mode = options.tools.as_ref().map(|tools| Arc::clone(&tools.mode));
        let handle = SessionHandle {
            tx,
            active: Arc::clone(&active),
            budget: Arc::clone(&budget),
            bypass_mode,
        };
        Ok((
            Self {
                store,
                session,
                provider_for_turn,
                sink,
                rx,
                active,
                budget,
                tools: options.tools,
                prompt_sources: options.prompt_sources,
                prompt_snapshot,
                project_name,
                network: options.network,
                announced_skills: std::collections::HashSet::new(),
            },
            handle,
        ))
    }

    /// Loads a session that can run guarded file tools and save checkpoints.
    ///
    /// # Errors
    /// Returns an error when the id is absent or the store cannot read it.
    pub fn new_with_tool_runtime(
        store: Store,
        id: &SessionId,
        provider_for_turn: ProviderFactory,
        sink: Arc<dyn crate::EventSink>,
        tools: ToolRuntime,
    ) -> Result<(Self, SessionHandle), SessionActorError> {
        Self::new_with_options(
            store,
            id,
            provider_for_turn,
            sink,
            SessionActorOptions {
                tools: Some(tools),
                prompt_sources: PromptSources::default(),
                network: None,
            },
        )
    }
}

fn load_snapshot(
    store: &Store,
    id: &SessionId,
    sources: &PromptSources,
) -> Result<Option<tether_store::PromptSnapshot>, SessionActorError> {
    if let Some(snapshot) = store.load_prompt_snapshot(id)? {
        return Ok(Some(snapshot));
    }
    let mut snapshot = prompt::capture_snapshot(sources).map_err(|_| SessionActorError::Prompt)?;
    snapshot.created_at = super::now_ms();
    store.save_prompt_snapshot(
        id,
        &snapshot.memory_block,
        &snapshot.vault_block,
        &snapshot.skills_block,
        snapshot.created_at,
    )?;
    Ok(Some(snapshot))
}
