//! Local repository operations.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use gix::bstr::ByteSlice;
use gix::hash::ObjectId;
use gix::status::Item as StatusItem;
use gix::status::index_worktree::Item as WorktreeItem;
use thiserror::Error;

/// A cancellation flag used by blocking git operations.
#[derive(Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Creates a token which is not cancelled.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Returns whether cancellation was requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    pub(crate) fn atomic(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.0)
    }
}

/// A caller-supplied commit identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Author and committer name.
    pub name: String,
    /// Author and committer email.
    pub email: String,
}

/// A file state in the index or worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum FileState {
    /// Added.
    Added,
    /// Modified.
    Modified,
    /// Deleted.
    Deleted,
    /// Renamed.
    Renamed,
    /// Untracked.
    Untracked,
    /// Conflicted.
    Conflicted,
}

/// A path and its two git state columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    /// Repository-relative path using `/`.
    pub path: String,
    /// State relative to `HEAD`.
    pub index_state: Option<FileState>,
    /// State relative to the index.
    pub worktree_state: Option<FileState>,
}

/// A commit identifier represented in hexadecimal.
pub type CommitId = String;

/// A commit summary returned by [`Repo::log`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    /// Commit id.
    pub id: CommitId,
    /// Parent ids, newest parent first.
    pub parents: Vec<CommitId>,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: String,
    /// Committer timestamp in Unix seconds.
    pub committer_time: i64,
    /// First message line.
    pub subject: String,
    /// Complete commit message.
    pub message: String,
}

/// A local branch and the commit it points to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInfo {
    /// Branch name without `refs/heads/`.
    pub name: String,
    /// Commit id, or `None` for an unborn branch.
    pub commit: Option<CommitId>,
}

/// A diff selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffSpec {
    /// Worktree compared with the index.
    Worktree,
    /// Index compared with `HEAD`.
    Staged,
    /// Worktree compared with `HEAD`.
    Head,
    /// Two revisions.
    Revisions {
        /// Older revision.
        old: String,
        /// Newer revision.
        new: String,
    },
}

/// Errors from local repository operations.
#[derive(Debug, Error)]
pub enum RepoError {
    /// The path is not a repository.
    #[error("path is not a git repository")]
    NotARepo,
    /// A relative path or branch name is invalid.
    #[error("invalid repository path or branch name")]
    InvalidName,
    /// The repository operation failed.
    #[error("git repository operation failed")]
    Git,
    /// File access failed.
    #[error("repository file operation failed")]
    Io,
    /// The worktree would overwrite changes in these paths.
    #[error("switch would overwrite changes in: {0}")]
    WouldOverwrite(String),
    /// A commit message was empty after trimming.
    #[error("commit message is empty")]
    EmptyMessage,
    /// No changes were staged.
    #[error("nothing is staged to commit")]
    EmptyCommit,
    /// The current branch cannot be deleted.
    #[error("the current branch cannot be deleted")]
    CurrentBranch,
    /// A requested ref was not found.
    #[error("git reference was not found")]
    RefNotFound,
}

/// A repository rooted at one project folder.
#[derive(Debug, Clone)]
pub struct Repo {
    root: PathBuf,
}

mod checkout;
mod diff;
mod helpers;
mod log;
mod ops;
mod stageable;

#[cfg(test)]
pub(crate) use helpers::create_branch_in;
pub use helpers::{branch_name_for_task, unique_branch_name, validate_branch_name};
