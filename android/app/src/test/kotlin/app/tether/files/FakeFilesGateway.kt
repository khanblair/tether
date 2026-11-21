package app.tether.files

/**
 * A files gateway for unit tests. Listings, files, and search results are set by the test. Each
 * call is recorded, and a folder or file in [failing] throws until the test removes it.
 */
class FakeFilesGateway : FilesGateway {
    /** Children by folder. The empty string is the project root. */
    val listings = mutableMapOf<String, DirListing>()

    /** Files by path. */
    val previews = mutableMapOf<String, FilePreview>()

    /** Folders and files that fail to load until the test removes them. */
    val failing = mutableSetOf<String>()

    /** The name the project has, or null when the id is unknown. */
    var name: String? = "Blog"

    /** The file name matches returned by every name search. */
    var nameHits: List<String> = emptyList()

    /** The text matches returned by every text search. */
    var textHits: List<FileTextHit> = emptyList()

    /** When set, every name search throws this. */
    var nameFailure: Exception? = null

    /** When set, every text search throws this. */
    var textFailure: Exception? = null

    /** Each listDir call as its folder and its show-ignored flag. */
    val listCalls = mutableListOf<Pair<String, Boolean>>()

    /** Each name search pattern, in order. */
    val nameCalls = mutableListOf<String>()

    /** Each text search text, in order. */
    val textCalls = mutableListOf<String>()

    /** Each read path, in order. */
    val readCalls = mutableListOf<String>()

    /** When set, every read throws this. */
    var readFailure: Exception? = null

    override fun projectName(projectId: String): String? = name

    override fun listDir(
        projectId: String,
        dir: String,
        showIgnored: Boolean,
    ): DirListing {
        listCalls += dir to showIgnored
        if (dir in failing) throw FilesFailure(FilesProblem.Unavailable)
        return listings[dir] ?: throw FilesFailure(FilesProblem.NotFound)
    }

    override fun readFile(
        projectId: String,
        path: String,
    ): FilePreview {
        readCalls += path
        readFailure?.let { throw it }
        if (path in failing) throw FilesFailure(FilesProblem.Unavailable)
        return previews[path] ?: throw FilesFailure(FilesProblem.NotFound)
    }

    override fun searchFileNames(
        projectId: String,
        pattern: String,
        limit: Int,
    ): List<String> {
        nameCalls += pattern
        nameFailure?.let { throw it }
        return nameHits
    }

    override fun searchFileText(
        projectId: String,
        text: String,
        limit: Int,
    ): List<FileTextHit> {
        textCalls += text
        textFailure?.let { throw it }
        return textHits
    }
}
