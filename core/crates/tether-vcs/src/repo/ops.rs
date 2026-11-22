use super::checkout::checkout_tree;
use super::diff::{tree_diff, tree_entry};
use super::helpers::{
    add_tree_change, add_worktree_change, commit_info, index_tree, index_tree_from,
    remove_from_index, stage_one, tree_for, unstage_one, validate_branch_name, validate_relative,
    worktree_diff,
};
use super::log::log_order;
use super::stageable::stageable_files;
use super::{
    BTreeMap, BranchInfo, ByteSlice, CommitId, CommitInfo, DiffSpec, Identity, Path, PathBuf,
    PreviousValue, Repo, RepoError, StatusEntry, StatusItem, fs,
};

impl Repo {
    /// Opens a repository rooted at `path`.
    ///
    /// # Errors
    /// Returns [`RepoError::NotARepo`] when no `.git` directory is present.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, RepoError> {
        let root = path.into();
        if !root.is_dir() || !root.join(".git").exists() || gix::open(&root).is_err() {
            return Err(RepoError::NotARepo);
        }
        Ok(Self { root })
    }

    /// Initializes an empty repository with `default_branch`.
    ///
    /// # Errors
    /// Returns an error when the destination is not empty or the branch is invalid.
    pub fn init(path: impl Into<PathBuf>, default_branch: &str) -> Result<Self, RepoError> {
        validate_branch_name(default_branch)?;
        let root = path.into();
        gix::ThreadSafeRepository::init(
            &root,
            gix::create::Kind::WithWorktree,
            gix::create::Options {
                destination_must_be_empty: Some(true),
                ..Default::default()
            },
        )
        .map_err(|_| RepoError::Git)?;
        if default_branch != "main" {
            fs::write(
                root.join(".git/HEAD"),
                format!("ref: refs/heads/{default_branch}\n"),
            )
            .map_err(|_| RepoError::Io)?;
        }
        Ok(Self { root })
    }

    /// Returns the repository root.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root
    }

    fn git(&self) -> Result<gix::Repository, RepoError> {
        gix::open(&self.root).map_err(|_| RepoError::NotARepo)
    }

    pub(crate) fn inner(&self) -> Result<gix::Repository, RepoError> {
        self.git()
    }

    /// Reads tracked and untracked status, excluding ignored paths.
    ///
    /// # Errors
    /// Returns an error when the index or worktree cannot be read.
    pub fn status(&self) -> Result<Vec<StatusEntry>, RepoError> {
        let repo = self.git()?;
        let mut found: BTreeMap<String, StatusEntry> = BTreeMap::new();
        let iter = repo
            .status(gix::progress::Discard)
            .map_err(|_| RepoError::Git)?
            .untracked_files(gix::status::UntrackedFiles::Files)
            .into_iter(Vec::<gix::bstr::BString>::new())
            .map_err(|_| RepoError::Git)?;
        for item in iter {
            let item = item.map_err(|_| RepoError::Git)?;
            match item {
                StatusItem::TreeIndex(change) => add_tree_change(&mut found, change),
                StatusItem::IndexWorktree(work) => add_worktree_change(&mut found, work),
            }
        }
        Ok(found
            .into_values()
            .filter(|e| e.index_state.is_some() || e.worktree_state.is_some())
            .collect())
    }

    /// Produces a bounded unified diff. The formatter uses three context lines.
    ///
    /// # Errors
    /// Returns an error when a revision or object cannot be read.
    pub fn diff(&self, spec: DiffSpec) -> Result<String, RepoError> {
        let repo = self.git()?;
        let (old, new) = match spec {
            DiffSpec::Worktree => (None, None),
            DiffSpec::Staged => (
                Some(
                    repo.head_tree_id_or_empty()
                        .map_err(|_| RepoError::Git)?
                        .detach(),
                ),
                Some(index_tree(&repo)?),
            ),
            DiffSpec::Head => (
                Some(
                    repo.head_tree_id_or_empty()
                        .map_err(|_| RepoError::Git)?
                        .detach(),
                ),
                None,
            ),
            DiffSpec::Revisions { old, new } => {
                (Some(tree_for(&repo, &old)?), Some(tree_for(&repo, &new)?))
            }
        };
        let mut out = match (old, new) {
            (Some(old), Some(new)) => tree_diff(&repo, old, new),
            (old, new) => worktree_diff(&repo, old, new)?,
        };
        cap_diff_text(&mut out);
        Ok(out)
    }

    /// Returns newest-first commits from `HEAD`.
    ///
    /// # Errors
    /// Returns an error when commit history cannot be decoded.
    #[expect(
        clippy::excessive_nesting,
        reason = "history filtering follows gix commit decoding"
    )]
    pub fn log(&self, limit: usize, path: Option<&str>) -> Result<Vec<CommitInfo>, RepoError> {
        if let Some(path) = path {
            validate_relative(path)?;
        }
        let repo = self.git()?;
        let head = repo.head_id().map_err(|_| RepoError::RefNotFound)?.detach();
        let mut out = Vec::new();
        for id in log_order(&repo, head)? {
            if out.len() >= limit {
                break;
            }
            let commit = repo.find_commit(id).map_err(|_| RepoError::Git)?;
            let decoded = commit.decode().map_err(|_| RepoError::Git)?;
            if path.is_none() || commit_changed_path(&repo, &commit, path.unwrap_or_default())? {
                out.push(commit_info(&commit, &decoded));
            }
        }
        Ok(out)
    }

    /// Returns the checked-out branch, or `None` when detached or unborn.
    ///
    /// # Errors
    /// Returns an error when `HEAD` cannot be read.
    pub fn current_branch(&self) -> Result<Option<String>, RepoError> {
        let repo = self.git()?;
        Ok(repo.head_name().map_err(|_| RepoError::Git)?.and_then(|n| {
            n.as_bstr()
                .to_str_lossy()
                .strip_prefix("refs/heads/")
                .map(str::to_owned)
        }))
    }

    /// Lists local branches in name order.
    ///
    /// # Errors
    /// Returns an error when references cannot be read.
    pub fn branches(&self) -> Result<Vec<BranchInfo>, RepoError> {
        let repo = self.git()?;
        let mut out = Vec::new();
        for reference in repo
            .references()
            .map_err(|_| RepoError::Git)?
            .local_branches()
            .map_err(|_| RepoError::Git)?
        {
            let mut reference = reference.map_err(|_| RepoError::Git)?;
            let name = reference
                .name()
                .as_bstr()
                .to_str_lossy()
                .strip_prefix("refs/heads/")
                .ok_or(RepoError::Git)?
                .to_owned();
            let commit = reference.peel_to_id().ok().map(|id| id.to_string());
            out.push(BranchInfo { name, commit });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Creates a local branch from a revision.
    ///
    /// # Errors
    /// Returns an error when the branch or start point is invalid.
    pub fn create_branch(&self, name: &str, start_point: &str) -> Result<(), RepoError> {
        validate_branch_name(name)?;
        let repo = self.git()?;
        let id = repo
            .rev_parse_single(start_point)
            .map_err(|_| RepoError::RefNotFound)?
            .detach();
        repo.reference(
            format!("refs/heads/{name}"),
            id,
            PreviousValue::MustNotExist,
            "branch: Created",
        )
        .map_err(|_| RepoError::Git)
        .map(|_| ())
    }

    /// Switches to a local branch after checking for overwrite risks.
    ///
    /// # Errors
    /// Returns [`RepoError::WouldOverwrite`] when changed, staged, or untracked paths would be
    /// replaced or lost.
    pub fn switch_branch(&self, name: &str) -> Result<(), RepoError> {
        validate_branch_name(name)?;
        let dirty: Vec<_> = self
            .status()?
            .into_iter()
            .filter_map(|e| {
                (e.worktree_state.is_some() || e.index_state.is_some()).then_some(e.path)
            })
            .collect();
        if !dirty.is_empty() {
            return Err(RepoError::WouldOverwrite(dirty.join(", ")));
        }
        let repo = self.git()?;
        let full = format!("refs/heads/{name}");
        repo.find_reference(full.as_str())
            .map_err(|_| RepoError::RefNotFound)?;
        let id = repo
            .rev_parse_single(full.as_str())
            .map_err(|_| RepoError::RefNotFound)?
            .detach();
        checkout_tree(&repo, id)?;
        fs::write(repo.git_dir().join("HEAD"), format!("ref: {full}\n")).map_err(|_| RepoError::Io)
    }

    /// Deletes a local branch other than the current branch.
    ///
    /// # Errors
    /// Returns [`RepoError::CurrentBranch`] for the checked-out branch.
    pub fn delete_branch(&self, name: &str) -> Result<(), RepoError> {
        validate_branch_name(name)?;
        if self.current_branch()?.as_deref() == Some(name) {
            return Err(RepoError::CurrentBranch);
        }
        let repo = self.git()?;
        let full = format!("refs/heads/{name}");
        let reference = repo
            .find_reference(full.as_str())
            .map_err(|_| RepoError::RefNotFound)?;
        reference.delete().map_err(|_| RepoError::Git)
    }

    /// Stages the given repository-relative paths.
    ///
    /// # Errors
    /// Returns an error when a path escapes the repository or the index cannot be written.
    pub fn stage(&self, paths: &[&str]) -> Result<(), RepoError> {
        for path in paths {
            validate_relative(path)?;
            stage_one(&self.git()?, path)?;
        }
        Ok(())
    }

    /// Stages what `git add -A` would: tracked files that changed or were deleted, and
    /// untracked files that no ignore rule excludes. It walks the worktree itself instead of
    /// trusting status, so a rename can never leave the new file out.
    ///
    /// # Errors
    /// Returns an error when the worktree or index cannot be read.
    pub fn stage_all(&self) -> Result<(), RepoError> {
        let repo = self.git()?;
        stageable_files(&repo, &self.root)?
            .iter()
            .try_for_each(|path| stage_one(&repo, path))?;
        self.drop_missing_tracked(&repo)
    }

    /// Removes index entries whose file is gone, such as the source of a rename.
    fn drop_missing_tracked(&self, repo: &gix::Repository) -> Result<(), RepoError> {
        let index = repo.open_index().map_err(|_| RepoError::Git)?;
        let tracked: Vec<String> = index
            .entries_with_paths_by_filter_map(|_p, e| {
                (e.stage() == gix::index::entry::Stage::Unconflicted).then_some(())
            })
            .map(|(p, ())| p.to_string())
            .collect();
        tracked
            .into_iter()
            .filter(|path| fs::symlink_metadata(self.root.join(path)).is_err())
            .try_for_each(|path| remove_from_index(repo, &path))
    }

    /// Restores the selected index paths from `HEAD`.
    ///
    /// # Errors
    /// Returns an error when the index cannot be written.
    pub fn unstage(&self, paths: &[&str]) -> Result<(), RepoError> {
        for path in paths {
            validate_relative(path)?;
            unstage_one(&self.git()?, path)?;
        }
        Ok(())
    }

    /// Commits the current index using `identity` for both signatures.
    ///
    /// # Errors
    /// Returns typed errors for an empty message or empty index delta.
    pub fn commit(&self, message: &str, identity: &Identity) -> Result<CommitId, RepoError> {
        if message.trim().is_empty() {
            return Err(RepoError::EmptyMessage);
        }
        let repo = self.git()?;
        let index = repo.open_index().map_err(|_| RepoError::EmptyCommit)?;
        if index_tree_from(&repo, &index)?
            == repo
                .head_tree_id_or_empty()
                .map_err(|_| RepoError::Git)?
                .detach()
        {
            return Err(RepoError::EmptyCommit);
        }
        let tree = index_tree_from(&repo, &index)?;
        let parent = repo.head_id().ok().map(gix::Id::detach);
        let now = gix::date::Time::now_local_or_utc();
        let zone = now.offset / 3600;
        let time_text = format!("{} {:+03}00", now.seconds, zone);
        let author = gix::actor::SignatureRef {
            name: identity.name.as_str().into(),
            email: identity.email.as_str().into(),
            time: &time_text,
        };
        let id = repo
            .commit_as(author, author, "HEAD", message, tree, parent)
            .map_err(|_| RepoError::Git)?;
        Ok(id.to_string())
    }
}

fn commit_changed_path(
    repo: &gix::Repository,
    commit: &gix::Commit<'_>,
    path: &str,
) -> Result<bool, RepoError> {
    let tree = commit.tree().map_err(|_| RepoError::Git)?.id;
    if commit.parent_ids().next().is_none() {
        return Ok(tree_entry(repo, tree, path).is_some());
    }
    for parent in commit.parent_ids() {
        let parent_tree = repo
            .find_commit(parent)
            .map_err(|_| RepoError::Git)?
            .tree()
            .map_err(|_| RepoError::Git)?
            .id;
        if tree_entry(repo, parent_tree, path) != tree_entry(repo, tree, path) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Cuts a diff to 1 MiB on a character boundary and says so.
fn cap_diff_text(out: &mut String) {
    const LIMIT: usize = 1024 * 1024;
    if out.len() <= LIMIT {
        return;
    }
    let mut end = LIMIT;
    while !out.is_char_boundary(end) {
        end -= 1;
    }
    out.truncate(end);
    out.push_str("\n[diff truncated at 1 MiB]\n");
}
