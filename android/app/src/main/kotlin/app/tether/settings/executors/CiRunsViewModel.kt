package app.tether.settings.executors

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** How many recent runs the list asks the core for. */
const val CI_RUN_LIMIT = 20

/** What the CI runs screen shows in its list area. */
sealed interface CiRunsContent {
    /** The first load has not returned yet. */
    data object Loading : CiRunsContent

    /** No project is open. */
    data object NoProject : CiRunsContent

    /** CI cannot run for this project yet. [readiness] says why. */
    data class NotReady(
        val readiness: CiReadiness,
    ) : CiRunsContent

    /** The runs could not be read. */
    data class Failed(
        val failure: ExecutorsFailure,
    ) : CiRunsContent

    /** The project has no runs yet. */
    data object Empty : CiRunsContent

    /** The most recent runs, newest first, as the core sent them. */
    data class Runs(
        val rows: List<CiRunRow>,
    ) : CiRunsContent
}

/** One row of the CI runs list. [startedAtMs] is null when the core's time text is not ISO. */
data class CiRunRow(
    val id: Long,
    val title: String,
    val branch: String,
    val state: CiRunState,
    val durationSeconds: Long,
    val startedAtMs: Long?,
    val htmlUrl: String,
)

/** The CI runs screen state. [refreshing] is true while a pull to refresh is running. */
@Immutable
data class CiRunsUiState(
    val content: CiRunsContent = CiRunsContent.Loading,
    val refreshing: Boolean = false,
)

/** Loads the recent CI runs of one project. Every core call runs on [io]. */
class CiRunsViewModel(
    private val gateway: ExecutorsGateway,
    private val projectId: String?,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(CiRunsUiState())

    /** The screen state. */
    val state: StateFlow<CiRunsUiState> = mutableState.asStateFlow()

    init {
        if (projectId == null) {
            mutableState.update { it.copy(content = CiRunsContent.NoProject) }
        } else {
            refresh()
        }
    }

    /** Reads the readiness and the recent runs again. */
    fun refresh() {
        val project = projectId ?: return
        mutableState.update { it.copy(refreshing = true) }
        viewModelScope.launch {
            val content = loadContent(project)
            mutableState.update { CiRunsUiState(content = content, refreshing = false) }
        }
    }

    private suspend fun loadContent(project: String): CiRunsContent {
        val readiness =
            call { gateway.ciReadiness(project) }.getOrElse { error ->
                return CiRunsContent.Failed(error.toExecutorsFailure())
            }
        return if (readiness != CiReadiness.Ready) {
            CiRunsContent.NotReady(readiness)
        } else {
            runsContent(call { gateway.ciRuns(project, CI_RUN_LIMIT) })
        }
    }

    /** The content for the result of a run list call. */
    private fun runsContent(result: Result<List<CiRunSummary>>): CiRunsContent =
        result.fold(
            onSuccess = { runs ->
                if (runs.isEmpty()) {
                    CiRunsContent.Empty
                } else {
                    CiRunsContent.Runs(runs.map { it.toRow() })
                }
            },
            onFailure = { error -> runsFailureContent(error.toExecutorsFailure()) },
        )

    private suspend fun <T> call(block: () -> T): Result<T> = callCore(io, block)

    companion object {
        /** Builds the runs model for [projectId], or for no project. */
        fun factory(
            gateway: ExecutorsGateway,
            projectId: String?,
        ) = viewModelFactory {
            initializer { CiRunsViewModel(gateway, projectId) }
        }
    }
}

/** The content for a failed run list. A missing sign-in or remote is a reason, not a failure. */
private fun runsFailureContent(failure: ExecutorsFailure): CiRunsContent =
    when (failure) {
        ExecutorsFailure.NotSignedIn -> CiRunsContent.NotReady(CiReadiness.NotSignedIn)
        ExecutorsFailure.NoRemote -> CiRunsContent.NotReady(CiReadiness.NoRemote)
        else -> CiRunsContent.Failed(failure)
    }

/** The list row for one run. */
internal fun CiRunSummary.toRow(): CiRunRow =
    CiRunRow(
        id = id,
        title = title,
        branch = branch,
        state = ciRunStateOf(status, conclusion),
        durationSeconds = durationSeconds,
        startedAtMs = parseStartedAt(startedAt),
        htmlUrl = htmlUrl,
    )
