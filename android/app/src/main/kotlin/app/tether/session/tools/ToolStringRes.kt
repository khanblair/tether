package app.tether.session.tools

import app.tether.R

/** The string resource that names a status. Exhaustive, so a new label needs a string. */
internal fun ToolStatusLabel.labelRes(): Int =
    when (this) {
        ToolStatusLabel.Proposed -> R.string.tools_status_proposed
        ToolStatusLabel.Allowed -> R.string.tools_status_allowed
        ToolStatusLabel.NeedsApproval -> R.string.tools_status_needs_approval
        ToolStatusLabel.Running -> R.string.tools_status_running
        ToolStatusLabel.Done -> R.string.tools_status_done
        ToolStatusLabel.Failed -> R.string.tools_status_failed
        ToolStatusLabel.TimedOut -> R.string.tools_status_timed_out
        ToolStatusLabel.Denied -> R.string.tools_status_denied
        ToolStatusLabel.Blocked -> R.string.tools_status_blocked
        ToolStatusLabel.Cancelled -> R.string.tools_status_cancelled
    }

/** The three forms of a verb: imperative, present, and past. */
internal class VerbRes(
    val imperative: Int,
    val present: Int,
    val past: Int,
) {
    /** The string for [tense]. */
    fun forTense(tense: ToolTense): Int =
        when (tense) {
            ToolTense.Imperative -> imperative
            ToolTense.Present -> present
            ToolTense.Past -> past
        }
}

/**
 * The verb of a title, for kinds that take a target. Kinds with a fixed name (git, pull request,
 * CI logs) are not in the table and return null, so they use [fixedTitleRes] instead.
 */
internal fun verbRes(kind: ToolTitleKind): VerbRes? = VERB_TABLE[kind]

/** Verb forms by kind. [ToolTitleKind.Unknown] is the fallback for tools not listed here. */
private val VERB_TABLE: Map<ToolTitleKind, VerbRes> =
    mapOf(
        ToolTitleKind.ReadFile to
            VerbRes(
                R.string.tools_verb_read_imperative,
                R.string.tools_verb_read_present,
                R.string.tools_verb_read_past,
            ),
        ToolTitleKind.WriteFile to
            VerbRes(
                R.string.tools_verb_write_imperative,
                R.string.tools_verb_write_present,
                R.string.tools_verb_write_past,
            ),
        ToolTitleKind.EditFile to
            VerbRes(
                R.string.tools_verb_edit_imperative,
                R.string.tools_verb_edit_present,
                R.string.tools_verb_edit_past,
            ),
        ToolTitleKind.DeleteFile to
            VerbRes(
                R.string.tools_verb_delete_imperative,
                R.string.tools_verb_delete_present,
                R.string.tools_verb_delete_past,
            ),
        ToolTitleKind.ListDir to
            VerbRes(
                R.string.tools_verb_list_imperative,
                R.string.tools_verb_list_present,
                R.string.tools_verb_list_past,
            ),
        ToolTitleKind.Search to
            VerbRes(
                R.string.tools_verb_search_imperative,
                R.string.tools_verb_search_present,
                R.string.tools_verb_search_past,
            ),
        ToolTitleKind.Run to
            VerbRes(
                R.string.tools_verb_run_imperative,
                R.string.tools_verb_run_present,
                R.string.tools_verb_run_past,
            ),
        ToolTitleKind.CiRun to
            VerbRes(
                R.string.tools_verb_ci_run_imperative,
                R.string.tools_verb_ci_run_present,
                R.string.tools_verb_ci_run_past,
            ),
        ToolTitleKind.Unknown to
            VerbRes(
                R.string.tools_verb_unknown_imperative,
                R.string.tools_verb_unknown_present,
                R.string.tools_verb_unknown_past,
            ),
    )

/** The fixed title of a kind that has no verb, such as "Git status". Null for verb kinds. */
internal fun fixedTitleRes(kind: ToolTitleKind): Int? =
    when (kind) {
        ToolTitleKind.GitStatus -> R.string.tools_title_git_status
        ToolTitleKind.GitDiff -> R.string.tools_title_git_diff
        ToolTitleKind.GitLog -> R.string.tools_title_git_log
        ToolTitleKind.GitCommit -> R.string.tools_title_git_commit
        ToolTitleKind.GitPush -> R.string.tools_title_git_push
        ToolTitleKind.GitBranch -> R.string.tools_title_git_branch
        ToolTitleKind.PrCreate -> R.string.tools_title_pr_create
        ToolTitleKind.PrStatus -> R.string.tools_title_pr_status
        ToolTitleKind.CiLogs -> R.string.tools_title_ci_logs
        ToolTitleKind.ReadFile,
        ToolTitleKind.WriteFile,
        ToolTitleKind.EditFile,
        ToolTitleKind.DeleteFile,
        ToolTitleKind.ListDir,
        ToolTitleKind.Search,
        ToolTitleKind.Run,
        ToolTitleKind.CiRun,
        ToolTitleKind.Unknown,
        -> null
    }
