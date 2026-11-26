package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavBackStackEntry
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.files.FileViewScreen
import app.tether.files.FileViewViewModel
import app.tether.files.FilesGateway
import app.tether.files.FilesScreen
import app.tether.files.FilesViewModel
import app.tether.messages.AppMessages

/** The file browser for one project. The argument is the project id, encoded. */
const val FILES_ROUTE = "files/{projectId}"

/** One file in the viewer. The arguments are the project id and the file's path, encoded. */
const val FILE_VIEW_ROUTE = "files/{projectId}/file/{path}"

/** The file browser of the project with [projectId]. */
fun filesPath(projectId: String) = "files/${Uri.encode(projectId)}"

/** The viewer for the file at [path] in the project with [projectId]. */
fun fileViewPath(
    projectId: String,
    path: String,
) = "files/${Uri.encode(projectId)}/file/${Uri.encode(path)}"

/** Registers the files area: the browser and the viewer. Both are read only. */
internal fun NavGraphBuilder.filesRoutes(
    gateway: FilesGateway,
    messages: AppMessages,
    nav: TetherNav,
) {
    composable(FILES_ROUTE) { entry ->
        val id = entry.argument("projectId")
        val files: FilesViewModel =
            viewModel(key = id, factory = FilesViewModel.factory(id, gateway))
        FilesScreen(
            viewModel = files,
            onBack = nav::back,
            onOpenFile = { nav.open(fileViewPath(id, it)) },
        )
    }
    composable(FILE_VIEW_ROUTE) { entry ->
        val id = entry.argument("projectId")
        val path = entry.argument("path")
        val viewer: FileViewViewModel =
            viewModel(key = "$id/$path", factory = FileViewViewModel.factory(id, path, gateway))
        FileViewScreen(
            viewModel = viewer,
            path = path,
            toasts = messages,
            onBack = nav::back,
            onPreview = { nav.open(previewPath(id, path)) },
        )
    }
}

private fun NavBackStackEntry.argument(name: String): String = arguments?.getString(name).orEmpty()
