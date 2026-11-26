package app.tether.session.agent

/**
 * What the tool cards need: the saved calls of the transcript, the ids of the cards that are
 * expanded, and the toggle for one id. A group uses the id "group:" plus the id of its first call.
 */
data class ToolCardState(
    val persisted: Map<String, ToolCallUi>,
    val expanded: Set<String>,
    val onToggle: (String) -> Unit,
    /** Opens the preview of a file a saved edit wrote. Null hides the Preview buttons. */
    val onPreview: ((String) -> Unit)? = null,
    /** The previewable page of each saved edit call, by call id. */
    val previewPaths: Map<String, String> = emptyMap(),
) {
    companion object {
        /** No saved calls and nothing expanded. */
        val Empty = ToolCardState(emptyMap(), emptySet(), onToggle = {})
    }
}
