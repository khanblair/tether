package app.tether.runs

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.providers.callCore
import app.tether.session.tools.RunExecutor
import app.tether.settings.executors.CiReadiness
import app.tether.settings.executors.CiRunRow
import app.tether.settings.executors.CiRunSummary
import app.tether.settings.executors.CiRunsContent
import app.tether.settings.executors.ExecutorsFailure
import app.tether.settings.executors.ExecutorsGateway
import app.tether.settings.executors.ciRunStateOf
import app.tether.settings.executors.parseStartedAt
import app.tether.settings.executors.toExecutorsFailure
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** How many local and GitHub rows a Runs load asks for. */
const val RUNS_LIMIT = 100

private data class LoadedRuns(
    val local: List<RunRow.Local>,
    val ci: CiLoad,
)

private sealed interface CiLoad {
    data object Loading : CiLoad

    data class Ready(
        val rows: List<CiRunRow>,
    ) : CiLoad

    data class NotReady(
        val readiness: CiReadiness,
    ) : CiLoad

    data class Failed(
        val failure: ExecutorsFailure,
    ) : CiLoad
}

/** Loads local and GitHub history, keeping filtering on the client. */
class RunsViewModel(
    private val runsGateway: RunsGateway,
    private val executorsGateway: ExecutorsGateway,
    projectId: String? = null,
    private val io: CoroutineDispatcher = Dispatchers.IO,
    private val processes: RunsProcesses = RegistryRunsProcesses,
) : ViewModel() {
    private val mutableState =
        MutableStateFlow(
            RunsUiState(
                projectId = projectId,
                content = if (projectId == null) RunsContent.NoProject else RunsContent.Loading,
                ciContent =
                    if (projectId == null) {
                        CiRunsContent.NoProject
                    } else {
                        CiRunsContent.Loading
                    },
            ),
        )
    private var loaded: LoadedRuns? = null
    private var loadJob: Job? = null

    /** The current project, filter, rows, and refresh state. */
    val state: StateFlow<RunsUiState> = mutableState.asStateFlow()

    init {
        if (projectId != null) refresh()
    }

    /** Chooses a filter without making another core call. */
    fun setFilter(filter: RunsFilter) {
        mutableState.update { current ->
            current.copy(
                filter = filter,
                content = contentFor(filter, loaded),
            )
        }
    }

    /** Changes the project and reads its history again. */
    fun selectProject(projectId: String?) {
        if (state.value.projectId == projectId) return
        loadJob?.cancel()
        loaded = null
        mutableState.value =
            state.value.copy(
                projectId = projectId,
                content = if (projectId == null) RunsContent.NoProject else RunsContent.Loading,
                ciContent =
                    if (projectId == null) {
                        CiRunsContent.NoProject
                    } else {
                        CiRunsContent.Loading
                    },
                refreshing = false,
            )
        if (projectId != null) refresh()
    }

    /** Reads both sources again while keeping the selected filter. */
    fun refresh() {
        val project = state.value.projectId
        if (project == null) {
            mutableState.update { it.copy(content = RunsContent.NoProject, refreshing = false) }
            return
        }
        loadJob?.cancel()
        mutableState.update { current ->
            current.copy(
                content =
                    if (loaded == null) {
                        RunsContent.Loading
                    } else {
                        contentFor(current.filter, loaded)
                    },
                ciContent = if (loaded == null) CiRunsContent.Loading else current.ciContent,
                refreshing = true,
            )
        }
        loadJob =
            viewModelScope.launch {
                val localResult = call { runsGateway.localRuns(project, RUNS_LIMIT) }
                val runs =
                    localResult.getOrElse { error ->
                        mutableState.update {
                            it.copy(
                                content = RunsContent.Failed(error.toRunsFailure()),
                                refreshing = false,
                            )
                        }
                        return@launch
                    }
                val live = matchLive(runs, call { processes.open() }.getOrDefault(emptyList()))
                val local = runs.map { RunRow.Local(it, live[it.callId]) }
                val ci = loadCi(project)
                loaded = LoadedRuns(local = local, ci = ci)
                val current = mutableState.value
                mutableState.value =
                    current.copy(
                        content = contentFor(current.filter, loaded),
                        ciContent = ci.toContent(),
                        refreshing = false,
                    )
            }
    }

    /** Stops the process of a live row, then reads the history again so the row updates. */
    fun stop(row: RunRow.Local) {
        val live = row.live ?: return
        viewModelScope.launch {
            call { processes.stop(live.sessionId, live.processId) }
            refresh()
        }
    }

    private suspend fun loadCi(projectId: String): CiLoad {
        val readiness = call { executorsGateway.ciReadiness(projectId) }
        return readiness.fold(
            onSuccess = { status ->
                if (status == CiReadiness.Ready) {
                    loadCiRuns(projectId)
                } else {
                    CiLoad.NotReady(status)
                }
            },
            onFailure = { error -> ciFailure(error.toExecutorsFailure()) },
        )
    }

    private suspend fun loadCiRuns(projectId: String): CiLoad {
        val result = call { executorsGateway.ciRuns(projectId, RUNS_LIMIT) }
        return result.fold(
            onSuccess = { rows -> CiLoad.Ready(rows.map(CiRunSummary::toRow)) },
            onFailure = { error -> ciFailure(error.toExecutorsFailure()) },
        )
    }

    private fun ciFailure(failure: ExecutorsFailure): CiLoad =
        when (failure) {
            ExecutorsFailure.NotSignedIn -> CiLoad.NotReady(CiReadiness.NotSignedIn)
            ExecutorsFailure.NoRemote -> CiLoad.NotReady(CiReadiness.NoRemote)
            else -> CiLoad.Failed(failure)
        }

    private fun contentFor(
        filter: RunsFilter,
        data: LoadedRuns?,
    ): RunsContent =
        when {
            data == null ->
                if (state.value.projectId == null) RunsContent.NoProject else RunsContent.Loading
            filter == RunsFilter.Ci -> data.ci.contentForCi()
            else -> localAndMergedContent(filter, data)
        }

    private fun localAndMergedContent(
        filter: RunsFilter,
        data: LoadedRuns,
    ): RunsContent {
        val local =
            when (filter) {
                RunsFilter.All -> data.local
                RunsFilter.Sandbox -> data.local.filter { it.run.executor == RunExecutor.Sandbox }
                RunsFilter.Linux -> data.local.filter { it.run.executor == RunExecutor.Linux }
                RunsFilter.Ci -> emptyList()
            }
        val rows =
            if (filter == RunsFilter.All) {
                mergeRuns(local, data.ci.rows())
            } else {
                local.sortedByDescending { it.startedAtMs }
            }
        return if (rows.isEmpty()) RunsContent.Empty else RunsContent.Runs(rows)
    }

    private fun CiLoad.contentForCi(): RunsContent =
        when (this) {
            CiLoad.Loading -> RunsContent.Loading
            is CiLoad.NotReady -> RunsContent.Empty
            is CiLoad.Failed -> RunsContent.Failed(RunsFailure.Unknown)
            is CiLoad.Ready -> {
                val rows = rows()
                if (rows.isEmpty()) RunsContent.Empty else RunsContent.Runs(rows)
            }
        }

    private fun CiLoad.toContent(): CiRunsContent =
        when (this) {
            CiLoad.Loading -> CiRunsContent.Loading
            is CiLoad.NotReady -> CiRunsContent.NotReady(readiness)
            is CiLoad.Failed -> CiRunsContent.Failed(failure)
            is CiLoad.Ready -> {
                val newest = rows.newestFirst()
                if (newest.isEmpty()) {
                    CiRunsContent.Empty
                } else {
                    CiRunsContent.Runs(newest)
                }
            }
        }

    private fun CiLoad.rows(): List<RunRow.Ci> =
        when (this) {
            is CiLoad.Ready ->
                rows
                    .map(CiRunRow::toRunRow)
                    .sortedByDescending { it.startedAtMs ?: Long.MIN_VALUE }
            else -> emptyList()
        }

    private suspend fun <T> call(block: () -> T): Result<T> = callCore(io, block)

    companion object {
        /** Builds the Runs model for an optional initial project. */
        fun factory(
            runsGateway: RunsGateway,
            executorsGateway: ExecutorsGateway,
            projectId: String? = null,
        ) = viewModelFactory {
            initializer { RunsViewModel(runsGateway, executorsGateway, projectId) }
        }
    }
}

private fun List<CiRunRow>.newestFirst(): List<CiRunRow> =
    sortedByDescending { it.startedAtMs ?: Long.MIN_VALUE }

private fun Throwable.toRunsFailure(): RunsFailure =
    if (this is RunsCallFailed) failure else RunsFailure.Unknown

private fun CiRunSummary.toRow(): CiRunRow =
    CiRunRow(
        id = id,
        title = title,
        branch = branch,
        state = ciRunStateOf(status, conclusion),
        durationSeconds = durationSeconds,
        startedAtMs = parseStartedAt(startedAt),
        htmlUrl = htmlUrl,
    )
