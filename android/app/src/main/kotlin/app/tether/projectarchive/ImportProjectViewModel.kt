package app.tether.projectarchive

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.home.ProjectNameCheck
import app.tether.home.ProjectNameProblem
import app.tether.home.checkProjectName
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** Where the Import a ZIP screen stands. */
enum class ImportPhase {
    /** No file is picked yet. */
    Idle,

    /** A file is picked and the person can name the project. */
    Picked,

    /** The ZIP is being copied and unpacked. Cancel is on. */
    Importing,

    /** The import did not finish. [ImportProjectUiState.reason] says why. */
    Failed,

    /** The project exists. The screen opens it. */
    Done,
}

/**
 * What the Import a ZIP screen shows. [nameProblem] is the reason under the name field, or null.
 * [nameUsable] is true when the trimmed name passes the same rules as New empty project.
 * [tooLarge] is true when the picked file is over the import limit. [projectId] is set when done.
 */
@Immutable
data class ImportProjectUiState(
    val phase: ImportPhase = ImportPhase.Idle,
    val fileName: String? = null,
    val sizeBytes: Long? = null,
    val tooLarge: Boolean = false,
    val name: String = "",
    val nameProblem: ProjectNameProblem? = null,
    val nameUsable: Boolean = false,
    val reason: ArchiveReason? = null,
    val projectId: String? = null,
) {
    /**
     * Import is on for a picked file, or a failed one to retry, when the name is usable and the
     * size is within the limit.
     */
    val canImport: Boolean
        get() =
            (phase == ImportPhase.Picked || phase == ImportPhase.Failed) &&
                nameUsable &&
                !tooLarge
}

/**
 * State holder for Import a ZIP. It copies the picked file, calls the core on [io], and deletes the
 * copy in every outcome. It has no Android types: the route turns picker results into [ArchiveUri].
 */
@Suppress("TooGenericExceptionCaught")
class ImportProjectViewModel(
    private val projects: ProjectArchiveGateway,
    private val files: ArchiveFiles,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(ImportProjectUiState())
    private var picked: ArchiveUri? = null

    @Volatile
    private var cancelRequested = false

    /** The screen state. */
    val state: StateFlow<ImportProjectUiState> = mutableState.asStateFlow()

    /** The person picked [uri]. Reads its name and size, and refuses it when over the limit. */
    fun onPicked(uri: ArchiveUri) {
        if (mutableState.value.phase == ImportPhase.Importing) return
        picked = uri
        viewModelScope.launch(io) { mutableState.value = pickedState(uri) }
    }

    /** The picker returned no file. The screen keeps what it showed. */
    fun onPickCancelled() = Unit

    /** Records the typed name and checks it. Ignored while an import runs or after it is done. */
    fun onNameChange(name: String) {
        val phase = mutableState.value.phase
        if (phase == ImportPhase.Importing || phase == ImportPhase.Done) return
        mutableState.update { it.withName(name) }
    }

    /** Starts the import of the picked file. Also used by Try again. */
    fun onImport() {
        val current = mutableState.value
        val uri = picked ?: return
        val check = checkProjectName(current.name) as? ProjectNameCheck.Valid
        if (!current.canImport || check == null) return
        cancelRequested = false
        mutableState.update { it.copy(phase = ImportPhase.Importing, reason = null) }
        viewModelScope.launch(io) { mutableState.value = runImport(uri, check.name) }
    }

    /** Asks the core to stop the import. The screen returns to Picked with no error. */
    fun onCancel() {
        if (mutableState.value.phase != ImportPhase.Importing) return
        cancelRequested = true
        mutableState.update { it.copy(phase = ImportPhase.Picked, reason = null) }
        viewModelScope.launch(io) { callQuietly { projects.cancelImport() } }
    }

    private fun pickedState(uri: ArchiveUri): ImportProjectUiState {
        val size = lookup { files.sizeBytes(uri) }
        val displayName = lookup { files.displayName(uri) }
        return ImportProjectUiState(
            phase = ImportPhase.Picked,
            fileName = displayName,
            sizeBytes = size,
            tooLarge = isOverImportLimit(size),
        ).withName(importNameFromFile(displayName))
    }

    private fun runImport(
        uri: ArchiveUri,
        name: String,
    ): ImportProjectUiState {
        val base = mutableState.value
        val temp =
            copyOrNull(uri)
                ?: return base.copy(phase = ImportPhase.Failed, reason = ArchiveReason.ReadFailed)
        return withTempFile(temp) { importFrom(temp, name, base) }
    }

    /** Runs [block], then deletes [temp] whatever the outcome. */
    private fun <T> withTempFile(
        temp: TempFile,
        block: () -> T,
    ): T =
        try {
            block()
        } finally {
            files.delete(temp)
        }

    private fun copyOrNull(uri: ArchiveUri): TempFile? =
        try {
            files.copyIn(uri)
        } catch (_: Exception) {
            null
        }

    private fun importFrom(
        temp: TempFile,
        name: String,
        base: ImportProjectUiState,
    ): ImportProjectUiState {
        if (cancelRequested) return base.copy(phase = ImportPhase.Picked)
        return when (val outcome = callCore { projects.import(temp.path, name) }) {
            is ArchiveResult.Succeeded ->
                base.copy(
                    phase = ImportPhase.Done,
                    projectId = outcome.value,
                )
            is ArchiveResult.Failed ->
                if (cancelRequested && outcome.reason == ArchiveReason.Cancelled) {
                    base.copy(phase = ImportPhase.Picked, reason = null)
                } else {
                    base.copy(phase = ImportPhase.Failed, reason = outcome.reason)
                }
        }
    }

    private fun ImportProjectUiState.withName(name: String): ImportProjectUiState {
        val check = checkProjectName(name)
        return copy(
            name = name,
            nameProblem = (check as? ProjectNameCheck.Invalid)?.problem,
            nameUsable = check is ProjectNameCheck.Valid,
        )
    }

    /** Runs a core call. Any exception becomes [ArchiveReason.Unavailable]. */
    private fun <T : Any> callCore(block: () -> ArchiveResult<T>): ArchiveResult<T> =
        try {
            block()
        } catch (_: Exception) {
            ArchiveResult.Failed(ArchiveReason.Unavailable)
        }

    /** Runs a call whose failure does not change the screen, such as cancel. */
    private fun callQuietly(block: () -> Unit) {
        runCatching(block)
    }

    private fun <T> lookup(block: () -> T?): T? =
        try {
            block()
        } catch (_: Exception) {
            null
        }
}
