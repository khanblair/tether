package app.tether.home

/** A project gateway for unit tests. It keeps projects in memory and records each name created. */
class FakeProjectGateway(
    initial: List<ProjectInfo> = emptyList(),
) : ProjectGateway {
    val projects = initial.toMutableList()
    val createdNames = mutableListOf<String>()

    /** When set, [create] returns this instead of making a project. */
    var createResult: CreateProjectResult? = null

    /** Throws this from the next [create] call, then clears itself. */
    var failNext: Exception? = null

    override fun list(): List<ProjectInfo> = projects.toList()

    override fun create(name: String): CreateProjectResult {
        failNext?.let {
            failNext = null
            throw it
        }
        createdNames += name
        createResult?.let { return it }
        val project = ProjectInfo("project-${projects.size + 1}", name, lastActivityMs = 1_000L)
        projects += project
        return CreateProjectResult.Created(project)
    }

    override fun load(id: String): ProjectInfo? = projects.firstOrNull { it.id == id }
}
