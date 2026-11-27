package app.tether.newsession

import app.tether.core.generated.NewSessionPreviewView
import app.tether.core.generated.SessionRuntime

/** The context that will be loaded when the first turn starts. */
data class NewSessionPreview(
    val instructionsFile: String?,
    val instructionsLines: Int,
    val memoryEntries: Int,
    val skillsAvailable: Int,
    val lastSessionTitle: String?,
    val lastSessionUpdatedAtMs: Long?,
)

/** Reads the pre-session context for a project. Calls block and must run off the main thread. */
interface NewSessionGateway {
    /** Reads the context that a new session in [projectId] will use. */
    fun preview(projectId: String): NewSessionPreview
}

/** The real gateway over the lazy UniFFI session runtime. */
class CoreNewSessionGateway(
    lazyRuntime: Lazy<SessionRuntime>,
) : NewSessionGateway {
    private val runtime: SessionRuntime by lazyRuntime

    override fun preview(projectId: String): NewSessionPreview =
        runtime.newSessionPreview(projectId).toPreview()
}

private fun NewSessionPreviewView.toPreview(): NewSessionPreview =
    NewSessionPreview(
        instructionsFile = instructionsFile,
        instructionsLines = instructionsLines.toInt(),
        memoryEntries = memoryEntries.toInt(),
        skillsAvailable = skillsAvailable.toInt(),
        lastSessionTitle = lastSessionTitle,
        lastSessionUpdatedAtMs = lastSessionUpdatedAtMs,
    )
