package app.tether.preview

/** What the preview shows: a project file, HTML held in memory, or an owned local server. */
sealed interface PreviewTarget {
    /** The file at [path] in the project with [projectId]. Its folder is the only one served. */
    data class File(
        val projectId: String,
        val path: String,
    ) : PreviewTarget

    /** HTML with no project behind it. Its requests all answer 404. */
    data class Inline(
        val html: String,
    ) : PreviewTarget

    /** A loopback server owned by a session process. */
    data class Server(
        val url: String,
        val processId: String,
    ) : PreviewTarget
}
