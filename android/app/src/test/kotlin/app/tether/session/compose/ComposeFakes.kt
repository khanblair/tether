package app.tether.session.compose

import app.tether.files.DirListing
import app.tether.files.FilePreview
import app.tether.files.FileTextHit
import app.tether.files.FilesFailure
import app.tether.files.FilesGateway

/** Commands for tests: four available, one that takes arguments, and one that is not available. */
val FixtureCommands =
    listOf(
        ComposerCommand("compact", "Summarize older context now", null, true, null),
        ComposerCommand("plan", "Switch to Plan mode", null, true, null),
        ComposerCommand("diff", "Open uncommitted changes", null, true, null),
        ComposerCommand("commit", "Write a message and commit", "[message]", true, null),
        ComposerCommand("review", "Second-model review", null, false, "a second model"),
    )

/** A commands gateway that returns [commands], or throws [failure] when it is set. */
class FakeCommandsGateway(
    private val commands: List<ComposerCommand> = FixtureCommands,
) : CommandsGateway {
    var failure: Exception? = null
    var listCalls = 0

    override fun commands(): List<ComposerCommand> {
        listCalls++
        failure?.let { throw it }
        return commands
    }

    override fun resolve(input: String): CommandResolution =
        CommandResolution.Unknown(input.removePrefix("/"), emptyList())
}

/** A files gateway whose file-name search returns [paths] and records each query it gets. */
class FakeFilesGateway(
    var paths: List<String> = emptyList(),
) : FilesGateway {
    var failure: FilesFailure? = null
    val searches = mutableListOf<Pair<String, Int>>()

    override fun projectName(projectId: String): String? = null

    override fun listDir(
        projectId: String,
        dir: String,
        showIgnored: Boolean,
    ): DirListing = DirListing(emptyList(), truncated = false)

    override fun readFile(
        projectId: String,
        path: String,
    ): FilePreview = error("readFile is not used by the composer")

    override fun searchFileNames(
        projectId: String,
        pattern: String,
        limit: Int,
    ): List<String> {
        searches += pattern to limit
        failure?.let { throw it }
        return paths
    }

    override fun searchFileText(
        projectId: String,
        text: String,
        limit: Int,
    ): List<FileTextHit> = emptyList()
}
