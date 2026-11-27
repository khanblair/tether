package app.tether.workspace

/** Paths where Tether keeps its local data. Values are paths only, never secret contents. */
data class WorkspaceDataPaths(
    val projects: String = "",
    val database: String = "",
    val secrets: String = "",
    val checkpoints: String = "",
    val memory: String = "",
    val skills: String = "",
    val vault: String = "",
    val crashReport: String = "",
)
