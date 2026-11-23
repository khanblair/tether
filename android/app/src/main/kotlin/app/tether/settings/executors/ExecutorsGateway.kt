package app.tether.settings.executors

/**
 * Executor and CI calls. Every call blocks, so callers run it off the main thread. A failed call
 * throws [ExecutorsCallFailed]. Progress reports may arrive on any thread.
 */
interface ExecutorsGateway {
    /** Whether Linux mode has its PRoot and root filesystem ready. */
    fun linuxReady(): Boolean

    /** The runtime pack versions this phone knows about. Empty until a pack is set up. */
    fun runtimePacks(): List<PackSummary>

    /** Installs one signed pack from [manifestUrl]. */
    fun installPack(
        packId: String,
        manifestUrl: String,
        report: ProgressReport,
    )

    /** Installs or repairs the signed Linux root filesystem from [manifestUrl]. */
    fun setupLinux(
        manifestUrl: String,
        report: ProgressReport,
    )

    /** Whether Linux mode may use the network for [projectId]. Off by default. */
    fun projectNetwork(projectId: String): Boolean

    /** Sets whether Linux mode may use the network for [projectId]. */
    fun setProjectNetwork(
        projectId: String,
        allowed: Boolean,
    )

    /** CI readiness for [projectId]. */
    fun ciReadiness(projectId: String): CiReadiness

    /** The most recent workflow runs for [projectId], at most [limit]. */
    fun ciRuns(
        projectId: String,
        limit: Int,
    ): List<CiRunSummary>
}
