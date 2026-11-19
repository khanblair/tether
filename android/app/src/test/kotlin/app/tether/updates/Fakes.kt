package app.tether.updates

import app.tether.core.generated.DownloadProgress
import app.tether.core.generated.ReleaseView
import app.tether.core.generated.UpdateCheckView
import java.io.File

fun releaseView(
    version: String = "0.0.3",
    sizeBytes: ULong = 1_000UL,
) = ReleaseView(
    version = version,
    tag = "v$version",
    notes = "Notes for $version",
    publishedAt = "2026-11-20T08:00:00Z",
    apkName = "tether-full-v$version.apk",
    apkSizeBytes = sizeBytes,
)

/** A gateway for unit tests. Each call records itself and can be made to throw once. */
class FakeUpdateGateway : UpdateGateway {
    var checkResult: UpdateCheckView = UpdateCheckView.UpToDate("0.0.2", "0.0.2")
    var downloadPath = "/files/tether-full-v0.0.3.apk"
    var failNext: Exception? = null
    var failCancel: Exception? = null
    var progressSteps: List<Pair<ULong, ULong>> = emptyList()
    val calls: MutableList<String> = java.util.Collections.synchronizedList(mutableListOf())
    var onCheck: (() -> Unit)? = null
    var onDownload: (() -> Unit)? = null
    var onCancel: (() -> Unit)? = null

    private fun record(call: String) {
        calls += call
        failNext?.let {
            failNext = null
            throw it
        }
    }

    override fun check(currentVersion: String): UpdateCheckView {
        record("check:$currentVersion")
        onCheck?.invoke()
        return checkResult
    }

    override fun download(
        version: String,
        progress: DownloadProgress,
    ): String {
        record("download:$version")
        onDownload?.invoke()
        progressSteps.forEach { (done, total) -> progress.onProgress(done, total) }
        return downloadPath
    }

    override fun cancel() {
        calls += "cancel"
        onCancel?.invoke()
        failCancel?.let { throw it }
    }
}

/** An installer for unit tests that reports whatever the test sets up. */
class FakeUpdateInstaller(
    override val installsInApp: Boolean = true,
) : UpdateInstaller {
    var canInstall = true
    var outcome: (InstallListener) -> Unit = { it.onSuccess() }
    var throwOnInstall: Exception? = null
    val calls = mutableListOf<String>()

    override fun canRequestInstalls() = canInstall

    override fun openInstallPermissionSettings() {
        calls += "settings"
    }

    override fun install(
        apk: File,
        listener: InstallListener,
    ) {
        calls += "install:${apk.path}"
        throwOnInstall?.let { throw it }
        outcome(listener)
    }

    override fun openStorePage() {
        calls += "store"
    }
}

/** An in-memory store. */
class FakeUpdateStore : UpdateStore {
    var last: Long? = null
    var available: String? = null

    override fun lastCheckMs() = last

    override fun availableVersion() = available

    override fun recordCheck(
        atMs: Long,
        available: String?,
    ) {
        last = atMs
        this.available = available
    }
}
