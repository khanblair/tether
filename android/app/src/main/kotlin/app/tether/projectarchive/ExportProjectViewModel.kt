package app.tether.projectarchive

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** Where the Export project sheet stands. */
enum class ExportPhase {
    /** The person chooses the switch and the Export button. */
    Editing,

    /** The core is writing the ZIP and the app is copying it to the chosen place. */
    Exporting,

    /** The export did not finish. [ExportProjectUiState.reason] says why. */
    Failed,

    /** The ZIP is saved at the chosen place. The sheet closes. */
    Done,
}

/**
 * What the Export project sheet shows. [suggestedName] is the file name offered to the file picker.
 * [includeIgnored] is off by default, so folders such as node_modules are left out.
 */
@Immutable
data class ExportProjectUiState(
    val projectName: String = "",
    val includeIgnored: Boolean = false,
    val phase: ExportPhase = ExportPhase.Editing,
    val reason: ArchiveReason? = null,
) {
    /** The file name offered to the file picker. */
    val suggestedName: String
        get() = exportFileName(projectName)
}

/**
 * State holder for Export project. The sheet asks the route for a destination; the route passes it
 * back as an [ArchiveUri]. The ZIP is written to the cache first, copied out, and deleted in every
 * outcome.
 */
@Suppress("TooGenericExceptionCaught")
class ExportProjectViewModel(
    private val projectId: String,
    projectName: String,
    private val projects: ProjectArchiveGateway,
    private val files: ArchiveFiles,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(ExportProjectUiState(projectName = projectName))
    private var destination: ArchiveUri? = null

    /** The screen state. */
    val state: StateFlow<ExportProjectUiState> = mutableState.asStateFlow()

    /** Turns "Include ignored files" on or off. Ignored while the export runs. */
    fun onIncludeIgnoredChange(includeIgnored: Boolean) {
        if (mutableState.value.phase != ExportPhase.Editing) return
        mutableState.update { it.copy(includeIgnored = includeIgnored) }
    }

    /** The person chose [uri] as the destination. Starts the export into it. */
    fun onDestinationChosen(uri: ArchiveUri) {
        if (mutableState.value.phase != ExportPhase.Editing) return
        destination = uri
        start(uri)
    }

    /** Runs the export again into the same destination. Only used after a failure. */
    fun onTryAgain() {
        val uri = destination ?: return
        if (mutableState.value.phase != ExportPhase.Failed) return
        start(uri)
    }

    private fun start(uri: ArchiveUri) {
        val include = mutableState.value.includeIgnored
        mutableState.update { it.copy(phase = ExportPhase.Exporting, reason = null) }
        viewModelScope.launch(io) { mutableState.value = runExport(uri, include) }
    }

    private fun runExport(
        uri: ArchiveUri,
        includeIgnored: Boolean,
    ): ExportProjectUiState {
        val base = mutableState.value
        val temp =
            try {
                files.reserveExport()
            } catch (_: Exception) {
                return base.copy(phase = ExportPhase.Failed, reason = ArchiveReason.WriteFailed)
            }
        try {
            return when (
                val outcome =
                    callCore {
                        projects.export(
                            projectId,
                            temp.path,
                            includeIgnored,
                        )
                    }
            ) {
                is ArchiveResult.Failed ->
                    base.copy(
                        phase = ExportPhase.Failed,
                        reason = outcome.reason,
                    )
                is ArchiveResult.Succeeded -> copyOut(temp, uri, base)
            }
        } finally {
            files.delete(temp)
        }
    }

    private fun copyOut(
        temp: TempFile,
        uri: ArchiveUri,
        base: ExportProjectUiState,
    ): ExportProjectUiState =
        try {
            files.copyOut(temp, uri)
            base.copy(phase = ExportPhase.Done)
        } catch (_: Exception) {
            base.copy(phase = ExportPhase.Failed, reason = ArchiveReason.WriteFailed)
        }

    private fun callCore(block: () -> ArchiveResult<Unit>): ArchiveResult<Unit> =
        try {
            block()
        } catch (_: Exception) {
            ArchiveResult.Failed(ArchiveReason.Unavailable)
        }
}
