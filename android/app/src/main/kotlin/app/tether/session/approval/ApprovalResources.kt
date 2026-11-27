package app.tether.session.approval

import androidx.compose.ui.graphics.vector.ImageVector
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.session.agent.AgentLoopFailure

/** The summary string of an approval kind, such as "Run command". */
internal fun ApprovalKind.labelRes(): Int =
    when (this) {
        ApprovalKind.Command -> R.string.approval_kind_command
        ApprovalKind.Server -> R.string.approval_kind_server
        ApprovalKind.ProcessStop -> R.string.approval_kind_process_stop
        ApprovalKind.Edit -> R.string.approval_kind_edit
        ApprovalKind.Delete -> R.string.approval_kind_delete
        ApprovalKind.Move -> R.string.approval_kind_move
        ApprovalKind.Network -> R.string.approval_kind_network
        ApprovalKind.Push -> R.string.approval_kind_push
        ApprovalKind.ForcePush -> R.string.approval_kind_force_push
        ApprovalKind.PullRequest -> R.string.approval_kind_pull_request
        ApprovalKind.GithubRepoCreate -> R.string.approval_kind_github_repo_create_private
        ApprovalKind.Other -> R.string.approval_kind_other
    }

/** The repository visibility wording for a GitHub repository approval. */
internal fun githubRepoApprovalLabelRes(target: String): Int =
    if (target.contains("(public)")) {
        R.string.approval_kind_github_repo_create_public
    } else {
        R.string.approval_kind_github_repo_create_private
    }

/** The icon that names the action of an approval kind. */
internal fun ApprovalKind.icon(): ImageVector =
    when (this) {
        ApprovalKind.Command -> TetherIcons.Terminal
        ApprovalKind.Server -> TetherIcons.Terminal
        ApprovalKind.ProcessStop -> TetherIcons.X
        ApprovalKind.Edit -> TetherIcons.FilePen
        ApprovalKind.Delete -> TetherIcons.Trash2
        ApprovalKind.Move -> TetherIcons.FileText
        ApprovalKind.Network -> TetherIcons.Globe
        ApprovalKind.Push, ApprovalKind.ForcePush -> TetherIcons.Upload
        ApprovalKind.PullRequest -> TetherIcons.GitBranch
        ApprovalKind.GithubRepoCreate -> TetherIcons.GitBranch
        ApprovalKind.Other -> TetherIcons.Package
    }

/** The message that says what a failed approval call means and what to do. */
internal fun AgentLoopFailure.messageRes(): Int =
    when (this) {
        AgentLoopFailure.NotFound -> R.string.approval_failure_not_found
        AgentLoopFailure.Busy -> R.string.approval_failure_busy
        AgentLoopFailure.Unavailable -> R.string.approval_failure_unavailable
        AgentLoopFailure.Other -> R.string.approval_failure_other
    }
