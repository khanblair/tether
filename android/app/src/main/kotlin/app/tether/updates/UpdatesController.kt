package app.tether.updates

import app.tether.core.generated.DownloadProgress
import app.tether.core.generated.ReleaseView
import app.tether.core.generated.UpdateCheckView
import app.tether.core.generated.UpdaterException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.util.concurrent.atomic.AtomicInteger

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
    private val fileExists: (String) -> Boolean = { File(it).isFile },
    isDebuggable: Boolean = false,
) {
    /** False in a debuggable build: nothing checks, restores, or offers an update. */
    private val enabled = updatesEnabled(isDebuggable)

    /** True while a check runs, so a background check and a manual one never overlap. */
    private val checkLock = Any()

    /** Guarded by [checkLock]. */
    private var checkInFlight = false

    /** Numbers each download, so a Cancel and its retries belong to one download only. */
    private val downloadId = AtomicInteger(0)

    /** The id of the download the user cancelled, or 0. */
    private val cancelledId = AtomicInteger(0)

    private val mutableState =
        MutableStateFlow(
            UpdatesUiState(
                currentVersion = currentVersion,
                installsInApp = installer.installsInApp,
                developmentBuild = !enabled,
            ),
        )

    /** The update state. */
    val state: StateFlow<UpdatesUiState> = mutableState.asStateFlow()

    /** Remembers the newer version an earlier launch found, so a banner can show at once. */
    fun restore() {
        if (!enabled) return
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
        if (!enabled) return
        scope.launch {
            val last = withContext(io) { store.lastCheckMs() }
            val due = last == null || clock() - last >= CHECK_INTERVAL_MS
            if (!due) return@launch
            val before = reserveCheck(showChecking = false) ?: return@launch
            runCheck(manual = false, before = before)
        }
    }

    /** Checks now and shows the result or the failure. */
    fun checkNow() {
        if (!enabled) return
        val before = reserveCheck(showChecking = true) ?: return
        scope.launch { runCheck(manual = true, before = before) }
    }

    /**
     * Takes the one check slot and, for a manual check, shows Checking. Returns the phase the
     * screen had, or null when the phase allows no check or a check already runs (a manual
     * request then shows Checking and lets the running check end it). Taking the slot, changing
     * the phase, publishing a result, and freeing the slot all happen under [checkLock], so a
     * request can never see a finished result and a busy slot at once.
     */
    private fun reserveCheck(showChecking: Boolean): UpdatePhase? =
        synchronized(checkLock) {
            val phase = state.value.phase
            if (phase !in CHECKABLE) return null
            val taken = !checkInFlight
            if (taken) checkInFlight = true
            if (showChecking) {
                mutableState.update { it.copy(phase = UpdatePhase.Checking, problem = null) }
            }
            if (taken) phase else null
        }

    /** Downloads the newer APK and verifies it. */
    fun download() {
        val current = state.value
        val release = current.release ?: return
        val canStart =
            current.phase == UpdatePhase.Available ||
                (current.phase == UpdatePhase.Failed && current.apkPath == null)
        if (!canStart) return
        val id = downloadId.incrementAndGet()
        mutableState.update {
            it.copy(
                phase = UpdatePhase.Downloading,
                downloadedBytes = 0L,
                totalBytes = release.apkSizeBytes,
                problem = null,
            )
        }
        scope.launch { runDownload(release, id) }
    }

    /** Stops the download in flight. */
    fun cancelDownload() {
        if (state.value.phase != UpdatePhase.Downloading) return
        val id = downloadId.get()
        cancelledId.set(id)
        scope.launch {
            // The core's cancel reaches only a download that has already registered itself, so a
            // Cancel tapped while the download is still starting could be lost. Ask again until
            // this download's call has returned, and never longer: a newer download is not ours.
            while (downloadId.get() == id && state.value.phase == UpdatePhase.Downloading) {
                call { gateway.cancel() }
                delay(CANCEL_RETRY_MS)
            }
        }
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
        try {
            val result = call { gateway.check(currentVersion) }
            // The store is written before the lock, which only covers quick state changes.
            remember(knownVersionAfter(result))
            synchronized(checkLock) {
                // Free the slot first: a state collector that runs inline may ask for a new check
                // the moment the result is published, and it must find the slot free.
                checkInFlight = false
                result
                    .onSuccess { applyCheck(it) }
                    .onFailure { onCheckFailed(it, manual, before) }
            }
        } finally {
            synchronized(checkLock) { checkInFlight = false }
        }
    }

    /** The newer version to remember: the one found, none when up to date, else the old one. */
    private fun knownVersionAfter(result: Result<UpdateCheckView>): String? =
        result.fold(
            onSuccess = { (it as? UpdateCheckView.Available)?.release?.version },
            onFailure = { state.value.let { s -> s.release?.version ?: s.knownNewerVersion } },
        )

    private fun onCheckFailed(
        failure: Throwable,
        manual: Boolean,
        before: UpdatePhase,
    ) {
        mutableState.update {
            when {
                it.phase !in CHECK_RESULT_PHASES -> it
                // A manual request joined this check and shows the spinner: tell the user.
                manual || it.phase == UpdatePhase.Checking ->
                    it.copy(phase = UpdatePhase.Failed, problem = problemFor(failure))
                else -> it.copy(phase = before)
            }
        }
    }

    private fun applyCheck(view: UpdateCheckView) {
        mutableState.update {
            if (it.phase !in CHECK_RESULT_PHASES) {
                it
            } else {
                when (view) {
                    is UpdateCheckView.UpToDate ->
                        it.copy(
                            phase = UpdatePhase.UpToDate,
                            release = null,
                            knownNewerVersion = null,
                            problem = null,
                        )
                    is UpdateCheckView.Available ->
                        it.copy(
                            phase = UpdatePhase.Available,
                            release = summaryOf(view.release),
                            knownNewerVersion = null,
                            problem = null,
                        )
                }
            }
        }
    }

    /** Records that a check ran now, and the newer version known, if any. */
    private suspend fun remember(available: String?) {
        call { store.recordCheck(clock(), available) }
    }

    private suspend fun runDownload(
        release: ReleaseSummary,
        id: Int,
    ) {
        if (cancelledId.get() == id) {
            mutableState.update { it.copy(phase = UpdatePhase.Available) }
            return
        }
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
                // A Cancel that arrived too late to stop the call still wins. The verified file
                // stays in the cache, and the next download reuses it.
                val phase =
                    if (cancelledId.get() ==
                        id
                    ) {
                        UpdatePhase.Available
                    } else {
                        UpdatePhase.Ready
                    }
                val apk = if (phase == UpdatePhase.Ready) path else null
                mutableState.update { it.copy(phase = phase, apkPath = apk) }
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
                    scope.launch { reportInstallFailure(path, problemFor(failure)) }
                }
            }
        scope.launch {
            if (!withContext(io) { fileExists(path) }) {
                reportInstallFailure(path, UpdateProblem.DownloadMissing)
                return@launch
            }
            call { installer.install(File(path), listener) }
                .onFailure { listener.onFailure(InstallFailure.Other) }
        }
    }

    /**
     * Shows an install failure. When the downloaded file is gone (the cache was cleared while the
     * app kept running), forget its path so the screen offers a new download, not a retry that
     * cannot work.
     */
    private suspend fun reportInstallFailure(
        path: String,
        problem: UpdateProblem,
    ) {
        val present = withContext(io) { fileExists(path) }
        mutableState.update {
            it.copy(
                phase = UpdatePhase.Failed,
                problem = if (present) problem else UpdateProblem.DownloadMissing,
                apkPath = if (present) it.apkPath else null,
            )
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
        const val CANCEL_RETRY_MS = 100L
        val CHECKABLE =
            setOf(UpdatePhase.Idle, UpdatePhase.UpToDate, UpdatePhase.Available, UpdatePhase.Failed)

        /** Phases in which a finished check may change what the screen shows. */
        val CHECK_RESULT_PHASES = CHECKABLE + UpdatePhase.Checking
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
