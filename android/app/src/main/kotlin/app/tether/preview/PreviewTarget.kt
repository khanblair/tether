package app.tether.preview

/** What the preview shows: one file of a project, or HTML held in memory. */
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
}
