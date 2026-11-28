package app.tether.settings.executors

import app.tether.core.generated.CiRunView
import app.tether.core.generated.CiRuntime
import app.tether.core.generated.CiRuntimeException
import app.tether.core.generated.ExecutorProgress
import app.tether.core.generated.ExecutorRuntimeException
import app.tether.core.generated.SessionRuntime

/**
 * The real gateway over the UniFFI [SessionRuntime] (executor calls) and [CiRuntime] (CI calls).
 * Every call blocks and maps the core's errors to [ExecutorsCallFailed].
 */
class CoreExecutorsGateway(
    private val sessionRuntime: Lazy<SessionRuntime>,
    private val ciRuntime: Lazy<CiRuntime>,
) : ExecutorsGateway {
    override fun linuxReady(): Boolean =
        guardExecutors { sessionRuntime.value.executorAvailability().linuxReady }

    override fun runtimePacks(): List<PackSummary> =
        guardExecutors {
            sessionRuntime.value.runtimePacks().map { pack ->
                PackSummary(
                    id = pack.id,
                    version = pack.version,
                    installed = pack.installed,
                    sizeBytes = pack.sizeBytes.toLong(),
                )
            }
        }

    override fun installPack(
        packId: String,
        manifestUrl: String,
        report: ProgressReport,
    ) {
        guardExecutors {
            sessionRuntime.value.installRuntimePack(packId, manifestUrl, report.toCore())
        }
    }

    override fun setupLinux(
        manifestUrl: String,
        report: ProgressReport,
    ) {
        guardExecutors { sessionRuntime.value.setupLinuxMode(manifestUrl, report.toCore()) }
    }

    override fun projectNetwork(projectId: String): Boolean =
        guardExecutors { sessionRuntime.value.projectNetwork(projectId) }

    override fun setProjectNetwork(
        projectId: String,
        allowed: Boolean,
    ) {
        guardExecutors { sessionRuntime.value.setProjectNetwork(projectId, allowed) }
    }

    override fun ciReadiness(projectId: String): CiReadiness =
        guardCi {
            ciRuntime.value.ciStatus(projectId).let { status ->
                ciReadinessOf(
                    ready = status.ready,
                    workflowPresent = status.workflowPresent,
                    workflowCurrent = status.workflowCurrent,
                    reason = status.reason,
                )
            }
        }

    override fun ciRuns(
        projectId: String,
        limit: Int,
    ): List<CiRunSummary> =
        guardCi {
            ciRuntime.value.listCiRuns(projectId, limit.toUInt()).map { it.toSummary() }
        }

    /** The core's error message can hold the GitHub detail, so it is never kept or shown. */
    @Suppress("SwallowedException")
    private inline fun <T> guardExecutors(block: () -> T): T =
        try {
            block()
        } catch (error: ExecutorRuntimeException) {
            throw ExecutorsCallFailed(error.toFailure(), error.detail())
        }

    /** The core's error message can hold the GitHub detail, so it is never kept or shown. */
    @Suppress("SwallowedException")
    private inline fun <T> guardCi(block: () -> T): T =
        try {
            block()
        } catch (error: CiRuntimeException) {
            throw ExecutorsCallFailed(error.toFailure())
        }
}

/** Passes byte progress from the core to a plain [ProgressReport], without UniFFI types. */
private fun ProgressReport.toCore(): ExecutorProgress {
    val report = this
    return object : ExecutorProgress {
        override fun onProgress(
            downloaded: ULong,
            total: ULong,
        ) {
            report(downloaded.toLong(), total.toLong())
        }
    }
}

private fun CiRunView.toSummary(): CiRunSummary =
    CiRunSummary(
        id = id.toLong(),
        title = title,
        branch = branch,
        status = status,
        conclusion = conclusion,
        startedAt = startedAt,
        durationSeconds = durationSeconds.toLong(),
        htmlUrl = htmlUrl,
    )
