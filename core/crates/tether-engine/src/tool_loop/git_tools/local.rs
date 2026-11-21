use std::sync::Arc;

use serde_json::Value;
use tether_vcs::{DiffSpec, FileState, Repo, RepoError};
use tokio_util::sync::CancellationToken;

use super::{
    CommitGate, CommitInput, GateFinding, GateSeverity, cap_diff, optional_bool, optional_string,
    optional_usize, string,
};

pub(super) fn execute(
    root: &std::path::Path,
    name: &str,
    payload: &Value,
    identity: Arc<dyn super::GitIdentityReader>,
    gates: Vec<Arc<dyn CommitGate>>,
    cancel: CancellationToken,
) -> Result<String, String> {
    let repo = Repo::open(root).map_err(repo_error)?;
    if cancel.is_cancelled() {
        return Err("git operation cancelled".into());
    }
    match name {
        "git_status" => status(&repo),
        "git_diff" => diff(&repo, payload),
        "git_log" => log(&repo, payload),
        "git_branch" => branch(&repo, payload),
        "git_commit" => commit(&repo, payload, identity, gates),
        _ => Err("unknown git tool".into()),
    }
}

fn status(repo: &Repo) -> Result<String, String> {
    repo.status()
        .map(|entries| {
            entries
                .into_iter()
                .map(|entry| {
                    format!(
                        "{}{} {}",
                        state(entry.index_state),
                        state(entry.worktree_state),
                        entry.path
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map_err(repo_error)
}

fn diff(repo: &Repo, payload: &Value) -> Result<String, String> {
    let from = optional_string(payload, "from")?;
    let to = optional_string(payload, "to")?;
    let spec = match (from, to) {
        (Some(old), Some(new)) => DiffSpec::Revisions {
            old: old.to_owned(),
            new: new.to_owned(),
        },
        (None, None) if optional_bool(payload, "staged", false)? => DiffSpec::Staged,
        (None, None) => DiffSpec::Worktree,
        _ => return Err("from and to must be supplied together".into()),
    };
    let text = repo.diff(spec).map_err(repo_error)?;
    if let Some(path) = optional_string(payload, "path")? {
        Ok(filter_diff_path(&text, path))
    } else {
        Ok(cap_diff(text))
    }
}

fn log(repo: &Repo, payload: &Value) -> Result<String, String> {
    let limit = optional_usize(payload, "limit", 10)?;
    let path = optional_string(payload, "path")?;
    repo.log(limit, path)
        .map(|commits| {
            commits
                .into_iter()
                .map(|commit| format!("{} {}", commit.id, commit.subject))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map_err(repo_error)
}

fn branch(repo: &Repo, payload: &Value) -> Result<String, String> {
    let name = optional_string(payload, "name")?.unwrap_or("tether/work");
    let from = optional_string(payload, "from")?.unwrap_or("HEAD");
    repo.create_branch(name, from).map_err(repo_error)?;
    if optional_bool(payload, "switch", true)? {
        repo.switch_branch(name).map_err(repo_error)?;
    }
    Ok(format!("Created branch {name} from {from}"))
}

fn commit(
    repo: &Repo,
    payload: &Value,
    identity: Arc<dyn super::GitIdentityReader>,
    gates: Vec<Arc<dyn CommitGate>>,
) -> Result<String, String> {
    let message = string(payload, "message")?;
    let saved = identity
        .identity()
        .map_err(|error| error.to_owned())?
        .ok_or_else(|| "Sign in to GitHub or set a name and email in Settings first".to_owned())?;
    stage(repo, payload)?;
    let changed_paths = repo
        .status()
        .map_err(repo_error)?
        .into_iter()
        .filter(|entry| entry.index_state.is_some())
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    if changed_paths.is_empty() {
        return Err("nothing is staged to commit".into());
    }
    let input = CommitInput {
        message: message.to_owned(),
        staged_diff: repo.diff(DiffSpec::Staged).map_err(repo_error)?,
        changed_paths: changed_paths.clone(),
    };
    let findings = gates
        .iter()
        .flat_map(|gate| gate.check(&input))
        .collect::<Vec<_>>();
    let blocks = findings
        .iter()
        .filter(|finding| finding.severity == GateSeverity::Block)
        .collect::<Vec<_>>();
    if !blocks.is_empty() {
        return Err(format_findings("Fix these before committing:", &blocks));
    }
    let id = repo.commit(message, &saved).map_err(repo_error)?;
    let warnings = findings
        .iter()
        .filter(|finding| finding.severity == GateSeverity::Warn)
        .collect::<Vec<_>>();
    let mut output = format!(
        "Committed {id}: {}",
        message.lines().next().unwrap_or_default()
    );
    if !warnings.is_empty() {
        output.push('\n');
        output.push_str(&format_findings("Warnings:", &warnings));
    }
    Ok(output)
}

fn stage(repo: &Repo, payload: &Value) -> Result<(), String> {
    let Some(paths) = payload.get("paths") else {
        return repo.stage_all().map_err(repo_error);
    };
    let paths = paths.as_array().ok_or("paths must be an array")?;
    let paths = paths
        .iter()
        .map(|path| path.as_str().ok_or("paths must contain strings"))
        .collect::<Result<Vec<_>, _>>()?;
    repo.stage(&paths).map_err(repo_error)
}

fn format_findings(prefix: &str, findings: &[&GateFinding]) -> String {
    let mut output = prefix.to_owned();
    for finding in findings {
        output.push('\n');
        output.push_str(&format!("{}: {}", finding.rule_id, finding.message));
    }
    output
}

fn filter_diff_path(text: &str, path: &str) -> String {
    let marker = format!("diff --git a/{path} b/{path}");
    let mut chunks = text.split("diff --git ");
    let mut output = String::new();
    while let Some(chunk) = chunks.next() {
        if chunk.starts_with(marker.trim_start_matches("diff --git ")) {
            output.push_str("diff --git ");
            output.push_str(chunk);
        }
    }
    cap_diff(output)
}

fn state(state: Option<FileState>) -> char {
    match state {
        None => ' ',
        Some(FileState::Added) => 'A',
        Some(FileState::Modified) => 'M',
        Some(FileState::Deleted) => 'D',
        Some(FileState::Renamed) => 'R',
        Some(FileState::Untracked) => '?',
        Some(FileState::Conflicted) => 'U',
    }
}

fn repo_error(error: RepoError) -> String {
    error.to_string()
}
