package app.tether.updates

import app.tether.core.generated.DownloadProgress
import app.tether.core.generated.ReleaseView
import app.tether.core.generated.UpdateCheckView
import app.tether.core.generated.UpdaterException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

/**
 * State holder for the update flow: check, download and verify, install. It lives as long as the
 * app, so the home screen banner and the About screen show the same state.
 */
class UpdatesController(
    private val gateway: UpdateGateway,
    private val installer: UpdateInstaller,
    private val store: UpdateStore,
    private val currentVersion: String,
    private val scope: CoroutineScope,
    private val io: CoroutineDispatcher = Dispatchers.IO,
    private val clock: () -> Long = System::currentTimeMillis,
) {
    private val mutableState =
        MutableStateFlow(
            UpdatesUiState(
                currentVersion = currentVersion,
                installsInApp = installer.installsInApp,
            ),
        )

    /** The update state. */
    val state: StateFlow<UpdatesUiState> = mutableState.asStateFlow()

    /** Remembers the newer version an earlier launch found, so a banner can show at once. */
    fun restore() {
        scope.launch {
            val known = withContext(io) { store.availableVersion() }
            if (known != null && isNewerVersion(known, currentVersion)) {
                mutableState.update {
                    if (it.release ==
                        null
                    ) {
                        it.copy(knownNewerVersion = known)
                    } else {
                        it
                    }
                }
            }
        }
    }

    /** Checks in the background when the last check is over 12 hours old. Failures stay silent. */
    fun checkIfDue() {
        scope.launch {
            val last = withContext(io) { store.lastCheckMs() }
            val due = last == null || clock() - last >= CHECK_INTERVAL_MS
            val before = state.value.phase
            if (due && before in CHECKABLE) runCheck(manual = false, before = before)
        }
    }

    /** Checks now and shows the result or the failure. */
    fun checkNow() {
        if (state.value.phase !in CHECKABLE) return
        mutableState.update { it.copy(phase = UpdatePhase.Checking, problem = null) }
        scope.launch { runCheck(manual = true, before = UpdatePhase.Idle) }
    }

    /** Downloads the newer APK and verifies it. */
    fun download() {
        val current = state.value
        val release = current.release ?: return
        val canStart =
            current.phase == UpdatePhase.Available ||
                (current.phase == UpdatePhase.Failed && current.apkPath == null)
        if (!canStart) return
        mutableState.update {
            it.copy(
                phase = UpdatePhase.Downloading,
                downloadedBytes = 0L,
                totalBytes = release.apkSizeBytes,
                problem = null,
            )
        }
        scope.launch { runDownload(release) }
    }

    /** Stops the download in flight. */
    fun cancelDownload() {
        if (state.value.phase == UpdatePhase.Downloading) scope.launch { call { gateway.cancel() } }
    }

    /** Hands the verified APK to the system installer, or opens the Play page on the Play build. */
    fun install() {
        val current = state.value
        val path = current.apkPath ?: return
        if (current.phase !in INSTALLABLE) return
        when {
            !installer.installsInApp -> installer.openStorePage()
            !installer.canRequestInstalls() ->
                mutableState.update {
                    it.copy(phase = UpdatePhase.NeedsPermission, problem = null)
                }
            else -> startInstall(path)
        }
    }

    /** Opens the system screen that allows this app to install packages. */
    fun openInstallSettings() = installer.openInstallPermissionSettings()

    /** The user came back from the system screen: continue when installs are now allowed. */
    fun onInstallPermissionResult() {
        if (state.value.phase == UpdatePhase.NeedsPermission && installer.canRequestInstalls()) {
            mutableState.update { it.copy(phase = UpdatePhase.Ready) }
        }
    }

    /** Opens the app's Google Play page. */
    fun openStorePage() = installer.openStorePage()

    private suspend fun runCheck(
        manual: Boolean,
        before: UpdatePhase,
    ) {
        call { gateway.check(currentVersion) }
            .onSuccess { applyCheck(it) }
            .onFailure { failure ->
                mutableState.update {
                    if (manual) {
                        it.copy(
                            phase = UpdatePhase.Failed,
                            problem = problemFor(failure),
                        )
                    } else {
                        it.copy(phase = before)
                    }
                }
            }
    }

    private suspend fun applyCheck(view: UpdateCheckView) {
        when (view) {
            is UpdateCheckView.UpToDate -> {
                withContext(io) { store.recordCheck(clock(), null) }
                mutableState.update {
                    it.copy(
                        phase = UpdatePhase.UpToDate,
                        release = null,
                        knownNewerVersion = null,
                        problem = null,
                    )
                }
            }
            is UpdateCheckView.Available -> {
                val release = summaryOf(view.release)
                withContext(io) { store.recordCheck(clock(), release.version) }
                mutableState.update {
                    it.copy(
                        phase = UpdatePhase.Available,
                        release = release,
                        knownNewerVersion = null,
                        problem = null,
                    )
                }
            }
        }
    }

    private suspend fun runDownload(release: ReleaseSummary) {
        val sink =
            object : DownloadProgress {
                override fun onProgress(
                    downloaded: ULong,
                    total: ULong,
                ) {
                    mutableState.update {
                        it.copy(
                            downloadedBytes = downloaded.toLong(),
                            totalBytes = total.toLong(),
                        )
                    }
                }
            }
        call { gateway.download(release.version, sink) }
            .onSuccess { path ->
                mutableState.update { it.copy(phase = UpdatePhase.Ready, apkPath = path) }
            }.onFailure { failure ->
                mutableState.update {
                    if (failure is UpdaterException.Cancelled) {
                        it.copy(phase = UpdatePhase.Available)
                    } else {
                        it.copy(phase = UpdatePhase.Failed, problem = problemFor(failure))
                    }
                }
            }
    }

    private fun startInstall(path: String) {
        mutableState.update { it.copy(phase = UpdatePhase.Installing, problem = null) }
        val listener =
            object : InstallListener {
                override fun onConfirmationShown() = Unit

                override fun onSuccess() {
                    mutableState.update { it.copy(phase = UpdatePhase.Installed) }
                }

                override fun onFailure(failure: InstallFailure) {
                    mutableState.update {
                        it.copy(
                            phase = UpdatePhase.Failed,
                            problem = problemFor(failure),
                        )
                    }
                }
            }
        scope.launch {
            call { installer.install(File(path), listener) }
                .onFailure { listener.onFailure(InstallFailure.Other) }
        }
    }

    private suspend fun <T> call(block: () -> T): Result<T> =
        try {
            Result.success(withContext(io) { block() })
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (
            @Suppress("TooGenericExceptionCaught") failure: Exception,
        ) {
            Result.failure(failure)
        }

    private companion object {
        const val CHECK_INTERVAL_MS = 12L * 60L * 60L * 1000L
        val CHECKABLE =
            setOf(UpdatePhase.Idle, UpdatePhase.UpToDate, UpdatePhase.Available, UpdatePhase.Failed)
        val INSTALLABLE = setOf(UpdatePhase.Ready, UpdatePhase.NeedsPermission, UpdatePhase.Failed)
    }
}

private fun summaryOf(view: ReleaseView) =
    ReleaseSummary(
        version = view.version,
        notes = view.notes,
        publishedAt = view.publishedAt,
        apkSizeBytes = view.apkSizeBytes.toLong(),
    )

internal fun problemFor(failure: Throwable): UpdateProblem =
    when (failure) {
        is UpdaterException.Network -> UpdateProblem.Network
        is UpdaterException.Unavailable -> UpdateProblem.NothingPublished
        is UpdaterException.Checksum -> UpdateProblem.ChecksumMismatch
        is UpdaterException.TooLarge -> UpdateProblem.TooLarge
        is UpdaterException.Storage -> UpdateProblem.Storage
        else -> UpdateProblem.BadResponse
    }

internal fun problemFor(failure: InstallFailure): UpdateProblem =
    when (failure) {
        InstallFailure.Cancelled -> UpdateProblem.InstallDeclined
        InstallFailure.Blocked -> UpdateProblem.InstallBlocked
        InstallFailure.Conflict -> UpdateProblem.InstallConflict
        InstallFailure.Incompatible -> UpdateProblem.InstallIncompatible
        InstallFailure.Invalid -> UpdateProblem.InstallInvalid
        InstallFailure.Storage -> UpdateProblem.InstallStorage
        InstallFailure.Other -> UpdateProblem.InstallFailed
    }
