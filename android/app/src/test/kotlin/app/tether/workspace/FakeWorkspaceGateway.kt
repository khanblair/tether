package app.tether.workspace

/** In-memory workspace gateway for view model tests. */
class FakeWorkspaceGateway(
    var savedLocation: WorkspaceLocation =
        WorkspaceLocation(
            defaultRoot = "/data/user/0/app/files/workspaces",
            newProjectRoot = null,
            effectiveRoot = "/data/user/0/app/files/workspaces",
        ),
    var nextProblem: WorkspaceProblem? = null,
) : WorkspaceGateway {
    val savedRoots = mutableListOf<String?>()

    override fun location(): WorkspaceLocation = savedLocation

    override fun setNewProjectRoot(path: String?): WorkspaceProblem? {
        savedRoots += path
        val problem = nextProblem
        if (problem == null) {
            savedLocation =
                savedLocation.copy(
                    newProjectRoot = path,
                    effectiveRoot = path ?: savedLocation.defaultRoot,
                )
        }
        return problem
    }
}
