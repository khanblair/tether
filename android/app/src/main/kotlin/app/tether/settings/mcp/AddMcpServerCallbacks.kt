package app.tether.settings.mcp

/**
 * What the add sheet can do. Each entry is a plain callback, so the sheet holds no state of its
 * own. [onHeaderNameChange] and the other header callbacks take the row's index.
 */
data class AddMcpServerCallbacks(
    val onNameChange: (String) -> Unit,
    val onUrlChange: (String) -> Unit,
    val onHeaderNameChange: (Int, String) -> Unit,
    val onHeaderValueChange: (Int, String) -> Unit,
    val onToggleReveal: (Int) -> Unit,
    val onAddHeader: () -> Unit,
    val onRemoveHeader: (Int) -> Unit,
    val onAdd: () -> Unit,
    val onDismissProblem: () -> Unit,
)
