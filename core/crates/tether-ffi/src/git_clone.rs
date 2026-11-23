use std::path::Path;
use std::sync::Arc;

use tether_engine::GithubClientFactory;
use tether_store::Store;
use tether_types::ProjectId;
use tether_vcs::{CloneRequest, Repo, parse_github_url, should_clone_shallow};

use crate::git::GitRuntime;
use crate::git_support::RuntimeGithub;
use crate::git_types::{GitRuntimeError, ShallowChoiceView};

pub(crate) fn clone_repository(
    runtime: &GitRuntime,
    url: &str,
    name: Option<String>,
    shallow: ShallowChoiceView,
    progress: Arc<dyn crate::CloneProgressView>,
) -> Result<String, GitRuntimeError> {
    let (branch, depth) = clone_choice(runtime, url, shallow)?;
    let project_name = name.unwrap_or_else(|| repository_name(url));
    let mut store = runtime.store()?;
    let project = store
        .create_project(&project_name, crate::git_support::now_ms())
        .map_err(|_| GitRuntimeError::Unavailable)?;
    let destination = Path::new(&runtime.workspaces_root).join(&project.path);
    if std::fs::create_dir_all(&runtime.workspaces_root).is_err() {
        return cleanup(&mut store, &project.id, &destination);
    }
    let cancel = Arc::new(tether_vcs::CancellationToken::new());
    *runtime
        .active_clone
        .lock()
        .map_err(|_| GitRuntimeError::Unavailable)? = Some(Arc::clone(&cancel));
    let result = run_clone(
        runtime,
        CloneJob {
            url: url.to_owned(),
            branch: branch.clone(),
            depth,
            destination: destination.clone(),
            progress,
            cancel: Arc::clone(&cancel),
        },
    );
    clear_clone(runtime, &cancel)?;
    if result.is_err() {
        return cleanup(&mut store, &project.id, &destination);
    }
    if store.set_project_remote(&project.id, url, &branch).is_err() {
        return cleanup(&mut store, &project.id, &destination);
    }
    Ok(project.id.to_string())
}

pub(crate) fn cancel_clone(runtime: &GitRuntime) {
    if let Ok(active) = runtime.active_clone.lock()
        && let Some(token) = active.as_ref()
    {
        token.cancel();
    }
}

#[cfg(test)]
pub(crate) fn test_clone_choice(
    runtime: &GitRuntime,
    url: &str,
    choice: ShallowChoiceView,
) -> Result<(String, Option<u32>), GitRuntimeError> {
    clone_choice(runtime, url, choice)
}

pub(crate) fn clone_for_factory(runtime: &GitRuntime) -> Result<GitRuntime, GitRuntimeError> {
    Ok(GitRuntime {
        database_path: runtime.database_path.clone(),
        workspaces_root: runtime.workspaces_root.clone(),
        vault: Arc::clone(&runtime.vault),
        runtime: Arc::clone(&runtime.runtime),
        oauth_client_id: std::sync::Mutex::new(
            runtime
                .oauth_client_id
                .lock()
                .map_err(|_| GitRuntimeError::Unavailable)?
                .clone(),
        ),
        device_grant: std::sync::Mutex::new(None),
        scopes: std::sync::Mutex::new(
            runtime
                .scopes
                .lock()
                .map_err(|_| GitRuntimeError::Unavailable)?
                .clone(),
        ),
        active_clone: std::sync::Mutex::new(None),
        http: std::sync::Mutex::new(
            runtime
                .http
                .lock()
                .map_err(|_| GitRuntimeError::Unavailable)?
                .clone(),
        ),
    })
}

fn clone_choice(
    runtime: &GitRuntime,
    url: &str,
    choice: ShallowChoiceView,
) -> Result<(String, Option<u32>), GitRuntimeError> {
    #[cfg(test)]
    if url.starts_with("file://") {
        return Ok((
            "main".into(),
            matches!(choice, ShallowChoiceView::Shallow).then_some(1),
        ));
    }
    let (owner, repo) = parse_github_url(url).ok_or(GitRuntimeError::InvalidInput)?;
    let client = RuntimeGithub(Arc::new(clone_for_factory(runtime)?))
        .client()
        .map_err(|_| GitRuntimeError::GithubNetwork)?;
    let info = runtime
        .runtime
        .block_on(client.get_repo(&owner, &repo))
        .map_err(|_| GitRuntimeError::GithubResponse)?;
    let shallow = match choice {
        ShallowChoiceView::Auto => should_clone_shallow(info.size_kb),
        ShallowChoiceView::Shallow => true,
        ShallowChoiceView::Full => false,
    };
    Ok((info.default_branch, shallow.then_some(1)))
}

struct CloneJob {
    url: String,
    branch: String,
    depth: Option<u32>,
    destination: std::path::PathBuf,
    progress: Arc<dyn crate::CloneProgressView>,
    cancel: Arc<tether_vcs::CancellationToken>,
}

fn run_clone(runtime: &GitRuntime, job: CloneJob) -> Result<Repo, GitRuntimeError> {
    let credentials = runtime.token().ok();
    let callback = CallbackCloneProgress(job.progress);
    runtime
        .runtime
        .block_on(runtime.runtime.spawn_blocking(move || {
            tether_vcs::clone(
                CloneRequest {
                    url: job.url,
                    dest: job.destination,
                    depth: job.depth,
                    branch: Some(job.branch),
                    credentials,
                },
                &callback,
                &job.cancel,
            )
        }))
        .map_err(|_| GitRuntimeError::Unavailable)?
        .map_err(|error| GitRuntimeError::Repository {
            detail: error.to_string(),
        })
}

fn cleanup(
    store: &mut Store,
    project_id: &ProjectId,
    destination: &Path,
) -> Result<String, GitRuntimeError> {
    drop(std::fs::remove_dir_all(destination));
    store
        .delete_project(project_id)
        .map_err(|_| GitRuntimeError::Unavailable)?;
    Err(GitRuntimeError::Repository {
        detail: "clone failed".into(),
    })
}

fn clear_clone(
    runtime: &GitRuntime,
    token: &Arc<tether_vcs::CancellationToken>,
) -> Result<(), GitRuntimeError> {
    let mut active = runtime
        .active_clone
        .lock()
        .map_err(|_| GitRuntimeError::Unavailable)?;
    if active
        .as_ref()
        .is_some_and(|current| Arc::ptr_eq(current, token))
    {
        *active = None;
    }
    Ok(())
}

fn repository_name(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or("project")
        .trim_end_matches(".git")
        .to_owned()
}

struct CallbackCloneProgress(Arc<dyn crate::CloneProgressView>);

impl tether_vcs::CloneProgress for CallbackCloneProgress {
    fn update(&self, phase: tether_vcs::ClonePhase, received: u64, total: Option<u64>) {
        self.0.on_progress(format!("{phase:?}"), received, total);
    }
}
