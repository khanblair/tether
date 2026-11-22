package app.tether.files

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * What the file viewer shows. [preview] is the core's read of the file, and [lines] are its text
 * split into lines. [problem] is set when the read failed.
 */
@Immutable
data class FileViewUiState(
    val loading: Boolean = true,
    val preview: FilePreview? = null,
    val lines: List<String> = emptyList(),
    val problem: FilesProblem? = null,
)

/** State holder for one file. The read runs on [io]; [retry] reads the file again. */
class FileViewViewModel(
    private val projectId: String,
    private val path: String,
    private val files: FilesGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(FileViewUiState())

    /** The viewer state. */
    val state: StateFlow<FileViewUiState> = mutableState.asStateFlow()

    init {
        retry()
    }

    /** Reads the file again. The last text stays out of view while the read runs. */
    fun retry() {
        mutableState.value = FileViewUiState(loading = true)
        viewModelScope.launch {
            val result = callCore(io) { files.readFile(projectId, path) }
            mutableState.value =
                result.fold(
                    onSuccess = { preview ->
                        FileViewUiState(
                            loading = false,
                            preview = preview,
                            lines = preview.text?.let(::sourceLines).orEmpty(),
                        )
                    },
                    onFailure = {
                        FileViewUiState(loading = false, problem = it.filesProblem())
                    },
                )
        }
    }

    companion object {
        /** Builds the viewer model for the file at [path] in the project with [projectId]. */
        fun factory(
            projectId: String,
            path: String,
            files: FilesGateway,
        ) = viewModelFactory {
            initializer { FileViewViewModel(projectId, path, files) }
        }
    }
}
