package app.tether.session.tools

import app.tether.session.agent.ToolCallUi
import app.tether.session.agent.ToolStatus

/** What a tool call does, in words people know. [Unknown] covers any tool not listed here. */
enum class ToolTitleKind {
    ReadFile,
    WriteFile,
    EditFile,
    DeleteFile,
    ListDir,
    Search,
    Run,
    RunServer,
    CiRun,
    ProcessList,
    ProcessLogs,
    ProcessStop,
    Preview,
    CiLogs,
    GitStatus,
    GitDiff,
    GitLog,
    GitCommit,
    GitPush,
    GitBranch,
    GitInit,
    GithubRepoCreate,
    PrCreate,
    PrStatus,
    GithubApi,
    Unknown,
}

/**
 * The verb form of a title. [Imperative] is for a call the user may still act on, [Present] is for
 * a call that runs, and [Past] is for a call that is over.
 */
enum class ToolTense {
    Imperative,
    Present,
    Past,
}

/** A title: the kind of action, its tense, and the object it acts on (may be empty). */
data class ToolTitle(
    val kind: ToolTitleKind,
    val tense: ToolTense,
    val target: String,
    val tool: String,
)

private val TITLE_KINDS: Map<String, ToolTitleKind> =
    mapOf(
        "read_file" to ToolTitleKind.ReadFile,
        "write_file" to ToolTitleKind.WriteFile,
        "edit_file" to ToolTitleKind.EditFile,
        "delete_file" to ToolTitleKind.DeleteFile,
        "list_dir" to ToolTitleKind.ListDir,
        "glob" to ToolTitleKind.Search,
        "grep" to ToolTitleKind.Search,
        "run" to ToolTitleKind.Run,
        "process_list" to ToolTitleKind.ProcessList,
        "process_logs" to ToolTitleKind.ProcessLogs,
        "process_stop" to ToolTitleKind.ProcessStop,
        "preview" to ToolTitleKind.Preview,
        "ci_run" to ToolTitleKind.CiRun,
        "ci_logs" to ToolTitleKind.CiLogs,
        "git_status" to ToolTitleKind.GitStatus,
        "git_diff" to ToolTitleKind.GitDiff,
        "git_log" to ToolTitleKind.GitLog,
        "git_commit" to ToolTitleKind.GitCommit,
        "git_push" to ToolTitleKind.GitPush,
        "git_branch" to ToolTitleKind.GitBranch,
        "git_init" to ToolTitleKind.GitInit,
        "github_repo_create" to ToolTitleKind.GithubRepoCreate,
        "pr_create" to ToolTitleKind.PrCreate,
        "pr_status" to ToolTitleKind.PrStatus,
        "github_api" to ToolTitleKind.GithubApi,
    )

/** The kind of action for a core tool name. An unknown name gives [ToolTitleKind.Unknown]. */
fun toolTitleKind(tool: String): ToolTitleKind = TITLE_KINDS[tool] ?: ToolTitleKind.Unknown

/**
 * The tense for a status: imperative while the user can still act (proposed or allowed, and
 * awaiting approval), present while it runs, and past once it is over.
 */
fun toolTense(status: ToolStatus): ToolTense =
    when (status) {
        ToolStatus.Proposed, ToolStatus.Allowed, ToolStatus.AwaitingApproval -> ToolTense.Imperative
        ToolStatus.Running -> ToolTense.Present
        ToolStatus.Succeeded,
        ToolStatus.Failed,
        ToolStatus.TimedOut,
        ToolStatus.Denied,
        ToolStatus.Blocked,
        ToolStatus.Cancelled,
        -> ToolTense.Past
    }

/**
 * The title of one call. For a `run` call the executor label is removed, because the run card
 * shows the executor as its own badge.
 */
fun toolTitle(call: ToolCallUi): ToolTitle {
    val baseKind = toolTitleKind(call.tool)
    val kind =
        if (baseKind == ToolTitleKind.Run && isBackgroundRun(call.target)) {
            ToolTitleKind.RunServer
        } else {
            baseKind
        }
    val target =
        when (kind) {
            ToolTitleKind.Run -> parseRunTarget(call.target).command
            ToolTitleKind.RunServer ->
                parseRunTarget(call.target).command.removeSuffix(BACKGROUND_SUFFIX)
            ToolTitleKind.ProcessList -> ""
            ToolTitleKind.ProcessLogs, ToolTitleKind.ProcessStop -> call.target
            ToolTitleKind.Preview -> previewTargetLabel(call.target)
            ToolTitleKind.GitInit -> gitInitTarget(call.target, call.output)
            ToolTitleKind.GithubRepoCreate -> githubRepoTarget(call.target, call.output)
            ToolTitleKind.GithubApi -> githubApiTarget(call.target)
            else -> call.target
        }
    return ToolTitle(kind, toolTense(call.status), target, call.tool)
}

/** Rebuilds the target of a saved tool call from its stored arguments and result. */
internal fun savedToolTarget(
    tool: String,
    argsJson: String,
    output: String,
): String =
    when (tool) {
        "run" -> savedRunTarget(argsJson)
        "process_list" -> ""
        "process_logs", "process_stop" -> stringArgument(argsJson, "id").orEmpty()
        "preview" ->
            stringArgument(argsJson, "path")
                ?: stringArgument(argsJson, "url")
                ?: outputPreviewTarget(output)
        "git_init" -> savedGitInitTarget(argsJson, output)
        "github_repo_create" ->
            githubRepoTarget(
                "",
                output,
                repositoryName = stringArgument(argsJson, "name"),
                isPrivate = booleanArgument(argsJson, "private"),
            )
        "github_api" ->
            listOfNotNull(stringArgument(argsJson, "method"), stringArgument(argsJson, "path"))
                .joinToString(" ")
        else -> ""
    }

/** The GitHub API target without the host: `GET /user/repos`, not the full URL. */
private fun githubApiTarget(target: String): String =
    target.replaceFirst(" https://api.github.com", " ")

private fun savedGitInitTarget(
    argsJson: String,
    output: String,
): String =
    gitInitTarget("", output).ifBlank {
        "on ${stringArgument(argsJson, "default_branch") ?: "main"}"
    }

private fun savedRunTarget(argsJson: String): String {
    val command = stringArgument(argsJson, "command").orEmpty()
    val executor = stringArgument(argsJson, "executor")
    val background = booleanArgument(argsJson, "background") == true
    val label = executor?.let { "[${it.lowercase()}] " }.orEmpty()
    val suffix = if (background) " (keeps running until stopped)" else ""
    return label + command + suffix
}

private fun gitInitTarget(
    target: String,
    output: String,
): String {
    val branch =
        target.removePrefix("on ").takeIf { it != target && it.isNotBlank() }
            ?: INIT_TARGET.find(target)?.groupValues?.get(1)
            ?: INIT_OUTPUT.find(output)?.groupValues?.get(1)
    return branch?.let { "on ${it.trim()}" }.orEmpty()
}

private fun githubRepoTarget(
    target: String,
    output: String,
    repositoryName: String? = null,
    isPrivate: Boolean? = null,
): String {
    if (target.contains(" · ")) return target
    val fromTarget = CREATE_TARGET.find(target)
    val repository = fromTarget?.groupValues?.get(1) ?: repositoryFromOutput(output)
    val visibility =
        fromTarget?.groupValues?.get(2) ?: if (isPrivate == false) "public" else "private"
    val base = repository ?: repositoryName.orEmpty()
    val web = webUrl(output)
    return listOfNotNull(base.takeIf { it.isNotBlank() }, visibility, web)
        .joinToString(" · ")
}

private fun webUrl(output: String): String? =
    WEB_URL
        .find(output)
        ?.groupValues
        ?.get(1)
        ?.trimEnd(',', '.', ')')

private fun repositoryFromOutput(output: String): String? =
    repositoryFromUrl(webUrl(output) ?: CLONE_URL.find(output)?.groupValues?.get(1))

private fun repositoryFromUrl(url: String?): String? =
    url?.trimEnd(',', '.', ')')?.removePrefix("https://github.com/")?.removeSuffix(".git")?.takeIf {
        it.count { character -> character == '/' } == 1
    }

private fun stringArgument(
    argsJson: String,
    key: String,
): String? =
    Regex("""\"$key\"\s*:\s*\"((?:[^\"\\\\]|\\\\.)*)\"""")
        .find(argsJson)
        ?.groupValues
        ?.get(1)
        ?.let(::unescapeJsonString)

private fun booleanArgument(
    argsJson: String,
    key: String,
): Boolean? =
    Regex("""\"$key\"\s*:\s*(true|false)""")
        .find(argsJson)
        ?.groupValues
        ?.get(1)
        ?.toBoolean()

private fun unescapeJsonString(raw: String): String =
    raw.replace("\\\"", "\"").replace("\\/", "/").replace("\\\\", "\\")

private val INIT_TARGET = Regex("""initialize a Git repository on (.+)""")
private val INIT_OUTPUT = Regex("""Initialized Git repository on (.+)""")
private val CREATE_TARGET = Regex("""create GitHub repository (.+) \((private|public)\)""")
private val WEB_URL = Regex("""web (https://\S+)""")
private val CLONE_URL = Regex("""clone (https://\S+)""")

private fun isBackgroundRun(target: String): Boolean = target.endsWith(BACKGROUND_SUFFIX)

private fun previewTargetLabel(target: String): String =
    if (target.startsWith("http://127.0.0.1:")) {
        val port = target.removePrefix("http://127.0.0.1:").takeWhile(Char::isDigit)
        if (port.isNotEmpty()) "the server on port $port" else target
    } else {
        target
    }

private fun outputPreviewTarget(output: String): String =
    PREVIEW_OUTPUT
        .find(output)
        ?.groupValues
        ?.get(1)
        .orEmpty()

private val PREVIEW_OUTPUT = Regex("""preview of (.+)""", RegexOption.IGNORE_CASE)

private const val BACKGROUND_SUFFIX = " (keeps running until stopped)"
