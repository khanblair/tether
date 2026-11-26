package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.navArgument
import app.tether.R
import app.tether.di.AppModule
import app.tether.messages.AppMessages
import app.tether.preview.INDEX_FILE
import app.tether.preview.InlinePreviews
import app.tether.preview.PreviewLoad
import app.tether.preview.PreviewScreen
import app.tether.preview.PreviewTarget
import app.tether.preview.PreviewUiState
import app.tether.preview.PreviewViewModel
import app.tether.preview.PreviewWebView

/** One file in the preview. The arguments are the project id and the file's path, both encoded. */
const val PREVIEW_ROUTE = "preview/{projectId}?path={path}"

/** One inline preview: HTML held in [InlinePreviews] under the id in the path. */
const val PREVIEW_INLINE_ROUTE = "preview/inline/{id}"

/** The preview of the file at [path] in the project with [projectId]. */
fun previewPath(
    projectId: String,
    path: String,
) = "preview/${Uri.encode(projectId)}?path=${Uri.encode(path)}"

/** The inline preview of the HTML held under [id]. */
fun previewInlinePath(id: String) = "preview/inline/${Uri.encode(id)}"

/** Opens the preview of [path] in the session's project. Without a project, nothing opens. */
internal fun TetherNav.previewFile(projectId: String?): (String) -> Unit =
    { path ->
        projectId?.let { open(previewPath(it, path)) }
    }

/** Opens an inline preview of the HTML given, holding it in [previews] under a new id. */
internal fun TetherNav.previewInline(previews: InlinePreviews): (String) -> Unit =
    { html ->
        open(previewInlinePath(previews.put(html)))
    }

/**
 * Registers the preview screens. The files come from [AppModule.previewFiles], and the inline HTML
 * from [AppModule.inlinePreviews].
 */
internal fun NavGraphBuilder.previewRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    val messages = module.messages
    val files = module.previewFiles
    val inlinePreviews = module.inlinePreviews

    composable(
        PREVIEW_ROUTE,
        arguments =
            listOf(
                navArgument("path") {
                    type = NavType.StringType
                    nullable = true
                    defaultValue = null
                },
            ),
    ) { entry ->
        val id = entry.arguments?.getString("projectId").orEmpty()
        val path = entry.arguments?.getString("path") ?: INDEX_FILE
        val viewModel: PreviewViewModel =
            viewModel(
                key = "$id/$path",
                factory =
                    PreviewViewModel.factory(PreviewTarget.File(id, path), files) {
                        module.executorsGateway.projectNetwork(id)
                    },
            )
        PreviewPage(viewModel, path.substringAfterLast('/'), nav, messages)
    }
    composable(PREVIEW_INLINE_ROUTE) { entry ->
        val id = entry.arguments?.getString("id").orEmpty()
        val html = inlinePreviews.get(id)
        if (html == null) {
            ExpiredPreview(nav)
        } else {
            val viewModel: PreviewViewModel =
                viewModel(
                    key = "inline/$id",
                    factory = PreviewViewModel.factory(PreviewTarget.Inline(html), files) { false },
                )
            PreviewPage(viewModel, title = null, nav = nav, messages = messages)
        }
    }
}

/** The preview screen for one view model. A null [title] shows the preview title. */
@Composable
private fun PreviewPage(
    viewModel: PreviewViewModel,
    title: String?,
    nav: TetherNav,
    messages: AppMessages,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val linkNotOpened = stringResource(R.string.preview_link_blocked)
    PreviewScreen(
        state = state,
        title = title,
        onBack = nav::back,
        onReload = viewModel::reload,
        onNetworkChange = viewModel::setNetworkAllowed,
        page = { load ->
            PreviewWebView(
                load = load,
                networkAllowed = viewModel::networkAllowed,
                readFile = viewModel::readFile,
                onBlocked = viewModel::onBlocked,
                onAddressBlocked = { url -> messages.show("$linkNotOpened $url") },
            )
        },
    )
}

/** An inline preview whose HTML was dropped from memory. It says so, with no page behind it. */
@Composable
private fun ExpiredPreview(nav: TetherNav) {
    PreviewScreen(
        state = PreviewUiState(load = PreviewLoad.Expired),
        title = null,
        onBack = nav::back,
        onReload = {},
        onNetworkChange = {},
        page = {},
    )
}
