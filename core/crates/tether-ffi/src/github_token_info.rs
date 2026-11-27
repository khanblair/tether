use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};

use crate::git::GitRuntime;
use crate::git_support::now_ms;
use crate::git_types::GitRuntimeError;

pub(crate) struct GithubTokenInfoRecord {
    pub(crate) kind: String,
    pub(crate) scopes: Vec<String>,
    pub(crate) login: String,
    pub(crate) can_read_repos: bool,
    pub(crate) checked_at: i64,
}

pub(crate) fn inspect_token(
    runtime: &GitRuntime,
    token: &str,
) -> Result<GithubTokenInfoRecord, GitRuntimeError> {
    let (status, user, scope_header) = runtime.http_bearer("https://api.github.com/user", token)?;
    if status == 401 {
        return Err(GitRuntimeError::GithubCredentials);
    }
    if !(200..300).contains(&status) {
        return Err(GitRuntimeError::GithubResponse);
    }
    let login = user
        .get("login")
        .and_then(serde_json::Value::as_str)
        .filter(|login| !login.is_empty())
        .ok_or(GitRuntimeError::GithubResponse)?
        .to_owned();
    let scopes = parse_scopes(scope_header.as_deref());
    let kind = match scope_header {
        Some(_) => "classic",
        None if token.starts_with("github_pat_") => "fine_grained",
        None => {
            return Err(GitRuntimeError::MissingScope {
                scope: "repo".into(),
            });
        }
    };
    let can_read_repos = probe_repos(runtime, token)?;
    Ok(GithubTokenInfoRecord {
        kind: kind.into(),
        scopes,
        login,
        can_read_repos,
        checked_at: now_ms(),
    })
}

fn parse_scopes(header: Option<&str>) -> Vec<String> {
    header
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
        .map(str::to_owned)
        .collect()
}

fn probe_repos(runtime: &GitRuntime, token: &str) -> Result<bool, GitRuntimeError> {
    let (status, _, _) =
        runtime.http_bearer("https://api.github.com/user/repos?per_page=1", token)?;
    match status {
        200..=299 => Ok(true),
        403 => Ok(false),
        401 => Err(GitRuntimeError::GithubCredentials),
        _ => Err(GitRuntimeError::GithubResponse),
    }
}

impl GitRuntime {
    pub(crate) fn save_github_token_info(
        &self,
        info: &GithubTokenInfoRecord,
    ) -> Result<(), GitRuntimeError> {
        let connection = self.github_info_connection()?;
        let scopes =
            serde_json::to_string(&info.scopes).map_err(|_| GitRuntimeError::Unavailable)?;
        connection
            .execute(
                "INSERT INTO github_token_info
                 (id, kind, scopes_json, login, can_read_repos, checked_at)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET kind = excluded.kind,
                     scopes_json = excluded.scopes_json, login = excluded.login,
                     can_read_repos = excluded.can_read_repos, checked_at = excluded.checked_at",
                params![
                    info.kind,
                    scopes,
                    info.login,
                    info.can_read_repos,
                    info.checked_at,
                ],
            )
            .map_err(|_| GitRuntimeError::Unavailable)?;
        Ok(())
    }

    pub(crate) fn github_token_info(
        &self,
    ) -> Result<Option<GithubTokenInfoRecord>, GitRuntimeError> {
        let connection = self.github_info_connection()?;
        connection
            .query_row(
                "SELECT kind, scopes_json, login, can_read_repos, checked_at
                 FROM github_token_info WHERE id = 1",
                [],
                read_token_info_row,
            )
            .optional()
            .map_err(|_| GitRuntimeError::Unavailable)
    }

    pub(crate) fn clear_github_token_info(&self) -> Result<(), GitRuntimeError> {
        let connection = self.github_info_connection()?;
        connection
            .execute("DELETE FROM github_token_info WHERE id = 1", [])
            .map_err(|_| GitRuntimeError::Unavailable)?;
        Ok(())
    }

    fn github_info_connection(&self) -> Result<Connection, GitRuntimeError> {
        let connection =
            Connection::open(&self.database_path).map_err(|_| GitRuntimeError::Unavailable)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|_| GitRuntimeError::Unavailable)?;
        Ok(connection)
    }
}

fn read_token_info_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GithubTokenInfoRecord> {
    let scopes_json: String = row.get(1)?;
    let scopes = serde_json::from_str(&scopes_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(GithubTokenInfoRecord {
        kind: row.get(0)?,
        scopes,
        login: row.get(2)?,
        can_read_repos: row.get(3)?,
        checked_at: row.get(4)?,
    })
}
