package app.tether.session.agent

/** Where a tool call stands, as the tool cards and the approval bar show it. */
enum class ToolStatus {
    /** The model asked for the call. Nothing has run yet. */
    Proposed,

    /** The call is allowed by policy or by an approval, and has not started. */
    Allowed,

    /** The call needs the user's approval before it can run. */
    AwaitingApproval,

    /** The call is running, and its output streams in. */
    Running,

    /** The call finished without error. */
    Succeeded,

    /** The call finished with an error. */
    Failed,

    /** The call ran past its time limit. */
    TimedOut,

    /** The user denied the call. */
    Denied,

    /** A rule or policy blocked the call. */
    Blocked,

    /** The call was cancelled before it finished. */
    Cancelled,
}

/**
 * One tool call of the current turn. [target] is the display text the core labelled for people.
 * [output] keeps only the newest part of the output. [outputTruncated] is true once older text
 * was cut.
 */
data class ToolCallUi(
    val id: String,
    val tool: String,
    val target: String,
    val status: ToolStatus,
    val output: String,
    val outputTruncated: Boolean,
)

/** A tool call that waits for the user's approval. */
data class ApprovalUi(
    val toolCallId: String,
    val tool: String,
    val target: String,
)

/** A question the agent asked the user. [options] are the suggested answers. */
data class QuestionUi(
    val toolCallId: String,
    val question: String,
    val options: List<String>,
    val allowFreeText: Boolean,
)

/** The state of one checklist item. */
enum class TodoStatus {
    /** Not started. */
    Pending,

    /** The item the agent is on now. */
    InProgress,

    /** Done. */
    Completed,
}

/** One checklist item. */
data class TodoUi(
    val text: String,
    val status: TodoStatus,
)

/**
 * A plan the agent proposed. [toolCallId] is null when the plan comes from a restart, because the
 * core does not tie a pending plan to a call. [steps], [files], [commands] and [openQuestions]
 * are the lists the plan card shows.
 */
data class PlanUi(
    val toolCallId: String?,
    val summary: String,
    val steps: List<String>,
    val files: List<String>,
    val commands: List<String>,
    val openQuestions: List<String>,
)

/** How serious one quality finding is. An unknown value from the core shows as [Info]. */
enum class FindingSeverity {
    /** A note. Nothing is wrong. */
    Info,

    /** A problem worth a look. */
    Warn,

    /** A problem that blocks the change. */
    Block,
}

/** One finding of a quality check. [line] is the line the core reported, clamped to Int. */
data class QualityFindingUi(
    val ruleId: String,
    val severity: FindingSeverity,
    val path: String,
    val line: Int,
    val message: String,
    val advice: String,
)

/**
 * The result of one quality check after a batch of tool calls. [info], [warn] and [block] count
 * the findings by severity. [skipped] counts changed files the check did not read.
 */
data class QualityUi(
    val toolCallId: String,
    val info: Int,
    val warn: Int,
    val block: Int,
    val findings: List<QualityFindingUi>,
    val skipped: Int,
)

/** Tokens and cost of the session so far. Cost is in micro-units of the account currency. */
data class UsageUi(
    val tokensIn: Long,
    val tokensOut: Long,
    val costMicros: Long,
)

/** The budget a warning or a stop applies to. */
enum class BudgetScope {
    /** The budget of this session. */
    Session,

    /** The budget of the day. */
    Day,
}

/** A notice shown above the transcript. Each kind has its own rule for when it clears. */
sealed interface AgentBanner {
    /** The spend passed [percent] of a budget. A newer warning of the same [scope] replaces it. */
    data class BudgetWarning(
        val scope: BudgetScope,
        val percent: Int,
        val spentMicros: Long,
        val limitMicros: Long,
    ) : AgentBanner

    /** The spend reached a budget, so the turns stop until the user continues. */
    data class BudgetReached(
        val scope: BudgetScope,
        val spentMicros: Long,
        val limitMicros: Long,
    ) : AgentBanner

    /** The network is down, and the turn waits for it. */
    data object NetworkPaused : AgentBanner

    /** The context was compacted. Only the latest one is kept. */
    data class Compacted(
        val tokensBefore: Int,
        val tokensAfter: Int,
    ) : AgentBanner
}
