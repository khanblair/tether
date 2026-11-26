package app.tether.session.agent

import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.preview.isPreviewable

/** The tools whose written file can be previewed: a file write or an edit of one. */
private val PREVIEW_TOOLS = setOf("write_file", "edit_file", "multi_edit")

/** The `"path"` member of a flat JSON object. Its value is a string without escaped quotes. */
private val PATH_MEMBER = Regex(""""path"\s*:\s*"((?:[^"\\]|\\.)*)"""")

/**
 * The page a saved call wrote, or null. Only a write or an edit of an HTML page counts, and its
 * path is read from the call's saved arguments.
 */
fun previewPathOf(call: MessagePartView.ToolCall): String? =
    previewablePath(call.tool, pathArgument(call.argsJson))

/** The pages that saved calls in [messages] wrote, by call id. */
fun savedPreviewPaths(messages: List<MessageView>): Map<String, String> =
    messages
        .flatMap { it.parts }
        .filterIsInstance<MessagePartView.ToolCall>()
        .mapNotNull { call -> previewPathOf(call)?.let { call.id to it } }
        .toMap()

/** [path] when [tool] is a previewable edit and [path] is an HTML page, else null. */
internal fun previewablePath(
    tool: String,
    path: String?,
): String? {
    if (tool !in PREVIEW_TOOLS) return null
    return path?.takeIf { isPreviewable(it) }
}

/** The path in a saved call's arguments, or null when there is none. */
internal fun pathArgument(argsJson: String): String? =
    PATH_MEMBER
        .find(argsJson)
        ?.groupValues
        ?.get(1)
        ?.let(::unescapeJsonString)

/** Undoes the escapes a JSON string may hold: `\"`, `\/` and `\\`. */
private fun unescapeJsonString(raw: String): String =
    raw.replace("\\\"", "\"").replace("\\/", "/").replace("\\\\", "\\")

/**
 * The tap of a card's Preview button: it opens [path] through [onPreview]. Null when the card has
 * no page or no handler.
 */
fun previewAction(
    path: String?,
    onPreview: ((String) -> Unit)?,
): (() -> Unit)? = if (path != null && onPreview != null) ({ onPreview(path) }) else null
