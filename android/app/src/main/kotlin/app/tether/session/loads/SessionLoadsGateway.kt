package app.tether.session.loads

import app.tether.core.generated.NewSessionPreviewView
import app.tether.core.generated.SessionRuntime

/** The context a session in one project loads when its first turn starts. */
data class SessionLoads(
    val instructionsFile: String?,
    val instructionsLines: Int,
    val memoryEntries: Int,
    val skillsAvailable: Int,
    val lastSessionTitle: String?,
    val lastSessionUpdatedAtMs: Long?,
)

/** Reads what a project loads into a session. Calls block, so run them off the main thread. */
interface SessionLoadsGateway {
    /** Reads the context that a session in [projectId] loads. */
    fun loads(projectId: String): SessionLoads
}

/** The real gateway over the lazy UniFFI session runtime. */
class CoreSessionLoadsGateway(
    lazyRuntime: Lazy<SessionRuntime>,
) : SessionLoadsGateway {
    private val runtime: SessionRuntime by lazyRuntime

    override fun loads(projectId: String): SessionLoads =
        runtime.newSessionPreview(projectId).toLoads()
}

private fun NewSessionPreviewView.toLoads(): SessionLoads =
    SessionLoads(
        instructionsFile = instructionsFile,
        instructionsLines = instructionsLines.toInt(),
        memoryEntries = memoryEntries.toInt(),
        skillsAvailable = skillsAvailable.toInt(),
        lastSessionTitle = lastSessionTitle,
        lastSessionUpdatedAtMs = lastSessionUpdatedAtMs,
    )
