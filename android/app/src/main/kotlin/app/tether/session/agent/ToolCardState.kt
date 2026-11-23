package app.tether.session.agent

/**
 * What the tool cards need: the saved calls of the transcript, the ids of the cards that are
 * expanded, and the toggle for one id. A group uses the id "group:" plus the id of its first call.
 */
data class ToolCardState(
    val persisted: Map<String, ToolCallUi>,
    val expanded: Set<String>,
    val onToggle: (String) -> Unit,
) {
    companion object {
        /** No saved calls and nothing expanded. */
        val Empty = ToolCardState(emptyMap(), emptySet()) {}
    }
}
