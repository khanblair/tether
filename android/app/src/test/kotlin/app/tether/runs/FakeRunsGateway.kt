package app.tether.runs

/** A local-runs source for view-model tests. */
class FakeRunsGateway(
    var runsByProject: Map<String, List<LocalRun>> = emptyMap(),
    var failure: RunsFailure? = null,
) : RunsGateway {
    val calls = mutableListOf<String>()
    val limits = mutableListOf<Int>()

    override fun localRuns(
        projectId: String,
        limit: Int,
    ): List<LocalRun> {
        calls += projectId
        limits += limit
        failure?.let { throw RunsCallFailed(it) }
        return runsByProject[projectId].orEmpty()
    }
}
