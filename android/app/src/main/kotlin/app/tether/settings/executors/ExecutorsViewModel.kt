package app.tether.settings.executors

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.linux.LinuxBackgroundController
import app.tether.linux.LinuxBackgroundMode
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger

/**
 * State for the Where commands run screen. Every core call runs on [io]. A long operation keeps
 * running when the screen rotates, because this model outlives the screen.
 *
 * A pack or Linux setup cannot be stopped in the core. [cancelPackInstall] only stops the screen
 * from waiting: the core's own run finishes in the background, and the list is read again when it
 * ends.
 */
class ExecutorsViewModel(
    private val gateway: ExecutorsGateway,
    private val build: ExecutorBuildConfig,
    private val projectId: String?,
    private val io: CoroutineDispatcher = Dispatchers.IO,
    private val linuxBackground: LinuxBackgroundController? = null,
    private val onLinuxReadyChanged: (Boolean) -> Unit = {},
) : ViewModel() {
    private val mutableState =
        MutableStateFlow(
            ExecutorsUiState(
                packInstallAvailable = build.packManifestUrl != null,
                packSource = manifestHost(build.packManifestUrl),
                linuxSetupInBuild = build.linuxSupported && build.rootfsManifestUrl != null,
                linuxBackgroundMode =
                    linuxBackground?.mode?.value ?: LinuxBackgroundMode.WhileRunning,
                linuxSource = manifestHost(build.rootfsManifestUrl),
                network = if (projectId == null) NetworkState.NoProject else NetworkState.Loading,
                ci = if (projectId == null) CiState.NoProject else CiState.Loading,
            ),
        )

    /** The screen state. */
    val state: StateFlow<ExecutorsUiState> = mutableState.asStateFlow()

    /** One counter per pack. A stale install, whose counter moved on, has its result dropped. */
    private val packTokens = ConcurrentHashMap<String, Int>()

    /** Counts Linux setups, so a stale setup's progress and result are dropped. */
    private val setupToken = AtomicInteger(0)

    /** Packs whose download the screen stopped waiting for. Their core run may still be going. */
    private val detached: MutableSet<String> = ConcurrentHashMap.newKeySet()

    init {
        loadPacks()
        loadLinux()
        loadNetwork()
        loadCi()
    }

    /** Reads the pack list again. */
    fun retryPacks() {
        loadPacks()
    }

    /** Installs [packId]. Does nothing while that pack is downloading or still detached. */
    fun installPack(packId: String) {
        val manifestUrl = build.packManifestUrl ?: return
        if (packId in detached || isBusy(mutableState.value.packOverrides[packId])) return
        val token = nextToken(packId)
        setOverride(packId, PackState.Downloading(downloaded = 0L, total = 0L))
        viewModelScope.launch {
            val result =
                call {
                    gateway.installPack(packId, manifestUrl) { downloaded, total ->
                        if (packTokens[packId] == token) {
                            setOverride(packId, progressState(downloaded, total))
                        }
                    }
                }
            if (packTokens[packId] != token) {
                // The screen stopped waiting. The core's run has ended, so the list is read again.
                detached.remove(packId)
                clearOverride(packId)
                loadPacks()
                return@launch
            }
            result
                .onSuccess {
                    clearOverride(packId)
                    loadPacks()
                }.onFailure { error ->
                    val failure = error.toExecutorsFailure()
                    val next =
                        if (failure == ExecutorsFailure.NotConfigured) {
                            PackState.NotAvailable
                        } else {
                            PackState.Failed(failure)
                        }
                    setOverride(packId, next)
                }
        }
    }

    /**
     * Stops the screen from waiting for [packId]'s download. The row shows that the download is
     * detached and offers no action until the core's run ends and the list is read again.
     */
    fun cancelPackInstall(packId: String) {
        if (mutableState.value.packOverrides[packId] !is PackState.Downloading) return
        nextToken(packId)
        detached += packId
        setOverride(packId, PackState.Detached)
    }

    /** Installs Linux mode. */
    fun installLinux() {
        runLinuxSetup()
    }

    /** Sets up Linux mode again, after it broke or to refresh its files. */
    fun repairLinux() {
        runLinuxSetup()
    }

    /** Saves the foreground-service rule selected in the Linux section. */
    fun setLinuxBackgroundMode(mode: LinuxBackgroundMode) {
        linuxBackground?.set(mode)
        mutableState.update { it.copy(linuxBackgroundMode = mode) }
    }

    /** Reads the network setting again. */
    fun retryNetwork() {
        mutableState.update { it.copy(network = NetworkState.Loading) }
        loadNetwork()
    }

    /** Sets network access for the open project, then reads back the saved value. */
    fun setNetworkAllowed(allowed: Boolean) {
        val project = projectId ?: return
        val current =
            (mutableState.value.network as? NetworkState.Loaded)?.takeIf { !it.saving } ?: return
        mutableState.update { it.copy(network = current.copy(saving = true, failure = null)) }
        viewModelScope.launch {
            call {
                gateway.setProjectNetwork(project, allowed)
                gateway.projectNetwork(project)
            }.onSuccess { saved ->
                mutableState.update { it.copy(network = NetworkState.Loaded(allowed = saved)) }
            }.onFailure { error ->
                val failure = error.toExecutorsFailure()
                mutableState.update {
                    it.copy(network = current.copy(saving = false, failure = failure))
                }
            }
        }
    }

    /** Reads the CI readiness again. */
    fun retryCi() {
        mutableState.update { it.copy(ci = CiState.Loading) }
        loadCi()
    }

    private fun loadPacks() {
        viewModelScope.launch {
            call { gateway.runtimePacks() }
                .onSuccess { packs ->
                    mutableState.update {
                        it.copy(packsLoaded = true, packCore = packs, packsFailure = null)
                    }
                }.onFailure { error ->
                    mutableState.update {
                        it.copy(
                            packsLoaded = true,
                            packCore = emptyList(),
                            packsFailure = error.toExecutorsFailure(),
                        )
                    }
                }
        }
    }

    private fun loadLinux() {
        if (!build.linuxSupported) {
            setLinux(LinuxState.Unavailable(LinuxUnavailable.ThisBuild))
            return
        }
        viewModelScope.launch {
            call { gateway.linuxReady() }
                .onSuccess { ready -> setLinux(readyState(ready)) }
                .onFailure { error -> setLinux(linuxFailureState(error.toExecutorsFailure())) }
        }
    }

    /** The state for a readiness check. With no manifest URL, a missing setup cannot be offered. */
    private fun readyState(ready: Boolean): LinuxState =
        when {
            ready -> LinuxState.Ready
            build.rootfsManifestUrl == null ->
                LinuxState.Unavailable(
                    LinuxUnavailable.NotConfigured,
                )
            else -> LinuxState.NotInstalled
        }

    private fun runLinuxSetup() {
        val manifestUrl = build.rootfsManifestUrl ?: return
        if (!build.linuxSupported || mutableState.value.linux is LinuxState.SettingUp) return
        val token = setupToken.incrementAndGet()
        setLinux(LinuxState.SettingUp(downloaded = 0L, total = 0L))
        viewModelScope.launch {
            val result =
                call {
                    gateway.setupLinux(manifestUrl) { downloaded, total ->
                        if (setupToken.get() == token) {
                            setLinux(LinuxState.SettingUp(downloaded, total))
                        }
                    }
                }
            if (setupToken.get() != token) return@launch
            result
                .onSuccess { loadLinux() }
                .onFailure { error -> setLinux(linuxFailureState(error.toExecutorsFailure())) }
        }
    }

    private fun loadNetwork() {
        val project = projectId ?: return
        viewModelScope.launch {
            call { gateway.projectNetwork(project) }
                .onSuccess { allowed ->
                    mutableState.update {
                        it.copy(
                            network = NetworkState.Loaded(allowed = allowed),
                        )
                    }
                }.onFailure { error ->
                    val failure = error.toExecutorsFailure()
                    mutableState.update { it.copy(network = NetworkState.LoadFailed(failure)) }
                }
        }
    }

    private fun loadCi() {
        val project = projectId ?: return
        viewModelScope.launch {
            call { gateway.ciReadiness(project) }
                .onSuccess { readiness ->
                    mutableState.update { it.copy(ci = CiState.Status(readiness)) }
                }.onFailure { error ->
                    val failure = error.toExecutorsFailure()
                    mutableState.update { it.copy(ci = CiState.Failed(failure)) }
                }
        }
    }

    private fun setLinux(linux: LinuxState) {
        mutableState.update { it.copy(linux = linux) }
        onLinuxReadyChanged(linux == LinuxState.Ready)
    }

    private fun setOverride(
        packId: String,
        state: PackState,
    ) {
        mutableState.update { it.copy(packOverrides = it.packOverrides + (packId to state)) }
    }

    private fun clearOverride(packId: String) {
        mutableState.update { it.copy(packOverrides = it.packOverrides - packId) }
    }

    private fun nextToken(packId: String): Int {
        val next = (packTokens[packId] ?: 0) + 1
        packTokens[packId] = next
        return next
    }

    private suspend fun <T> call(block: () -> T): Result<T> = callCore(io, block)

    companion object {
        /** Builds the executor model for the open project, or for no project. */
        fun factory(
            gateway: ExecutorsGateway,
            build: ExecutorBuildConfig,
            projectId: String?,
            linuxBackground: LinuxBackgroundController? = null,
            onLinuxReadyChanged: (Boolean) -> Unit = {},
        ) = viewModelFactory {
            initializer {
                ExecutorsViewModel(
                    gateway = gateway,
                    build = build,
                    projectId = projectId,
                    linuxBackground = linuxBackground,
                    onLinuxReadyChanged = onLinuxReadyChanged,
                )
            }
        }
    }
}

/** A download that is still running or being checked. Such a pack cannot start again. */
private fun isBusy(state: PackState?): Boolean =
    state is PackState.Downloading || state == PackState.Verifying

/** The row state for progress reports: Verifying once every byte has arrived. */
private fun progressState(
    downloaded: Long,
    total: Long,
): PackState =
    if (total > 0L && downloaded >= total) {
        PackState.Verifying
    } else {
        PackState.Downloading(downloaded, total)
    }

/** The Linux state after a failed check or setup. A missing build key means it cannot be done. */
private fun linuxFailureState(failure: ExecutorsFailure): LinuxState =
    if (failure == ExecutorsFailure.NotConfigured) {
        LinuxState.Unavailable(LinuxUnavailable.NotConfigured)
    } else {
        LinuxState.Broken(failure)
    }
