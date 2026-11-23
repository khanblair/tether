package app.tether.session.agent

import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView

/**
 * The tool calls saved in the transcript, by call id, in the order they were proposed. A call
 * takes its status and output from its result, which may sit in a later message. A call with no
 * result never finished, so it shows as [ToolStatus.Cancelled]. A persisted call has no display
 * target, so [ToolCallUi.target] is empty. The first call with an id wins.
 */
fun toolCallsFromMessages(messages: List<MessageView>): Map<String, ToolCallUi> {
    val results = LinkedHashMap<String, MessagePartView.ToolResult>()
    val calls = LinkedHashMap<String, MessagePartView.ToolCall>()
    for (message in messages) {
        for (part in message.parts) {
            when (part) {
                is MessagePartView.ToolResult -> results[part.toolCallId] = part
                is MessagePartView.ToolCall -> calls.putIfAbsent(part.id, part)
                else -> Unit
            }
        }
    }
    return calls.mapValues { (id, call) -> persistedCall(call, results[id]) }
}

/** Builds the card model of one saved call from its result, if there is one. */
private fun persistedCall(
    call: MessagePartView.ToolCall,
    result: MessagePartView.ToolResult?,
): ToolCallUi =
    ToolCallUi(
        id = call.id,
        tool = call.tool,
        target = "",
        status = result.toStatus(),
        output = result?.text.orEmpty(),
        outputTruncated = result?.resultRef != null,
    )

private fun MessagePartView.ToolResult?.toStatus(): ToolStatus =
    when {
        this == null -> ToolStatus.Cancelled
        isError -> ToolStatus.Failed
        else -> ToolStatus.Succeeded
    }

/** True for a tool message that holds only results. Its output shows on the call's card. */
fun MessageView.isToolResultOnly(): Boolean =
    role == RoleView.TOOL && parts.all { it is MessagePartView.ToolResult }

/** One piece of a message: a part to draw as it is, or a run of saved tool calls. */
sealed interface PartSegment {
    /** A part drawn as it is. */
    data class Part(
        val part: MessagePartView,
    ) : PartSegment

    /** Consecutive saved calls, to group with [groupCalls]. */
    data class Calls(
        val calls: List<ToolCallUi>,
    ) : PartSegment
}

/**
 * Splits the parts of a message into segments. A saved call joins the run of calls around it. A
 * result is dropped, since its output shows on the call. A call that is not in [persisted] is
 * live, so the live cards show it, and it is dropped here.
 */
fun segmentParts(
    parts: List<MessagePartView>,
    persisted: Map<String, ToolCallUi>,
): List<PartSegment> {
    val segments = mutableListOf<PartSegment>()
    val run = mutableListOf<ToolCallUi>()
    for (part in parts) {
        when (part) {
            is MessagePartView.ToolResult -> Unit
            is MessagePartView.ToolCall -> persisted[part.id]?.let { run += it }
            else -> {
                flushRun(segments, run)
                segments += PartSegment.Part(part)
            }
        }
    }
    flushRun(segments, run)
    return segments
}

private fun flushRun(
    segments: MutableList<PartSegment>,
    run: MutableList<ToolCallUi>,
) {
    if (run.isNotEmpty()) {
        segments += PartSegment.Calls(run.toList())
        run.clear()
    }
}
