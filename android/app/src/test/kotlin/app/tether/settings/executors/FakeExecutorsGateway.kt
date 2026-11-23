package app.tether.settings.executors

/**
 * Executor and CI calls kept in memory. Each call is recorded in [calls]. A failure field makes
 * that call throw [ExecutorsCallFailed]. Progress pairs are reported in order, before the call
 * returns, and [beforeReport] runs before each one so a test can read the state in between.
 */
class FakeExecutorsGateway(
    var linuxIsReady: Boolean = false,
    var availabilityFailure: ExecutorsFailure? = null,
    var packs: List<PackSummary> = emptyList(),
    var packsFailure: ExecutorsFailure? = null,
    var installProgress: List<Pair<Long, Long>> = emptyList(),
    var installFailure: ExecutorsFailure? = null,
    var setupProgress: List<Pair<Long, Long>> = emptyList(),
    var setupFailure: ExecutorsFailure? = null,
    var network: Boolean = false,
    var networkFailure: ExecutorsFailure? = null,
    var readiness: CiReadiness = CiReadiness.Ready,
    var readinessFailure: ExecutorsFailure? = null,
    var runs: List<CiRunSummary> = emptyList(),
    var runsFailure: ExecutorsFailure? = null,
) : ExecutorsGateway {
    /** Every call, in order, as a short label such as "install:python-3.12". */
    val calls = mutableListOf<String>()

    /** The limits passed to [ciRuns]. */
    val runLimits = mutableListOf<Int>()

    /** Runs before each progress report. */
    var beforeReport: (() -> Unit)? = null

    override fun linuxReady(): Boolean {
        calls += "linuxReady"
        throwIfSet(availabilityFailure)
        return linuxIsReady
    }

    override fun runtimePacks(): List<PackSummary> {
        calls += "runtimePacks"
        throwIfSet(packsFailure)
        return packs
    }

    override fun installPack(
        packId: String,
        manifestUrl: String,
        report: ProgressReport,
    ) {
        calls += "install:$packId"
        for ((downloaded, total) in installProgress) {
            beforeReport?.invoke()
            report(downloaded, total)
        }
        throwIfSet(installFailure)
        packs = packs.map { if (it.id == packId) it.copy(installed = true) else it }
    }

    override fun setupLinux(
        manifestUrl: String,
        report: ProgressReport,
    ) {
        calls += "setup"
        for ((downloaded, total) in setupProgress) {
            beforeReport?.invoke()
            report(downloaded, total)
        }
        throwIfSet(setupFailure)
        linuxIsReady = true
    }

    override fun projectNetwork(projectId: String): Boolean {
        calls += "projectNetwork"
        throwIfSet(networkFailure)
        return network
    }

    override fun setProjectNetwork(
        projectId: String,
        allowed: Boolean,
    ) {
        calls += "setProjectNetwork:$allowed"
        throwIfSet(networkFailure)
        network = allowed
    }

    override fun ciReadiness(projectId: String): CiReadiness {
        calls += "ciReadiness"
        throwIfSet(readinessFailure)
        return readiness
    }

    override fun ciRuns(
        projectId: String,
        limit: Int,
    ): List<CiRunSummary> {
        calls += "ciRuns"
        runLimits += limit
        throwIfSet(runsFailure)
        return runs
    }

    private fun throwIfSet(failure: ExecutorsFailure?) {
        if (failure != null) throw ExecutorsCallFailed(failure)
    }
}
