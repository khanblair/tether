package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
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
import app.tether.preview.encodeQueryValue
import app.tether.session.servers.ServerProcessRegistry
import app.tether.session.servers.ServerStatus

/** One file in the preview. The arguments are the project id and the file's path, both encoded. */
const val PREVIEW_ROUTE = "preview/{projectId}?path={path}"

/** One inline preview: HTML held in [InlinePreviews] under the id in the path. */
const val PREVIEW_INLINE_ROUTE = "preview/inline/{id}"

/** A server preview, addressed by its owned URL and process id. */
const val PREVIEW_SERVER_ROUTE = "preview/server?url={url}&process={process}"

/** The preview of the file at [path] in the project with [projectId]. */
fun previewPath(
    projectId: String,
    path: String,
) = "preview/${Uri.encode(projectId)}?path=${Uri.encode(path)}"

/** The inline preview of the HTML held under [id]. */
fun previewInlinePath(id: String) = "preview/inline/${Uri.encode(id)}"

/** The server preview route with both query values percent-encoded. */
fun previewServerPath(
    url: String,
    processId: String,
) = "preview/server?url=${encodeQueryValue(url)}&process=${encodeQueryValue(processId)}"

private const val SERVER_PREVIEW_KEY = "\u0000server:"

/** Encodes a server target for the session preview callback without changing its public type. */
fun serverPreviewKey(
    url: String,
    processId: String,
) = SERVER_PREVIEW_KEY + Uri.encode(url) + "|" + Uri.encode(processId)

/** Opens the preview of [path] in the session's project. Without a project, nothing opens. */
internal fun TetherNav.previewFile(projectId: String?): (String) -> Unit =
    { path ->
        if (path.startsWith(SERVER_PREVIEW_KEY)) {
            val values = path.removePrefix(SERVER_PREVIEW_KEY).split('|', limit = 2)
            if (values.size == 2) {
                open(previewServerPath(Uri.decode(values[0]), Uri.decode(values[1])))
            }
        } else {
            projectId?.let { open(previewPath(it, path)) }
        }
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
    previewFileRoute(module, nav)
    previewInlineRoute(module, nav)
    previewServerRoute(module, nav)
}

/** The file preview route: one project file, loaded through the project's files. */
private fun NavGraphBuilder.previewFileRoute(
    module: AppModule,
    nav: TetherNav,
) {
    val messages = module.messages
    val files = module.previewFiles
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
}

/** The inline preview route: HTML held in memory. A missing entry shows the expired state. */
private fun NavGraphBuilder.previewInlineRoute(
    module: AppModule,
    nav: TetherNav,
) {
    val messages = module.messages
    val files = module.previewFiles
    val inlinePreviews = module.inlinePreviews
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

/** The server preview route: a dev server's page, which stops when its process stops. */
private fun NavGraphBuilder.previewServerRoute(
    module: AppModule,
    nav: TetherNav,
) {
    val messages = module.messages
    val files = module.previewFiles
    composable(
        PREVIEW_SERVER_ROUTE,
        arguments =
            listOf(
                navArgument("url") { type = NavType.StringType },
                navArgument("process") { type = NavType.StringType },
            ),
    ) { entry ->
        val url = entry.arguments?.getString("url").orEmpty()
        val processId = entry.arguments?.getString("process").orEmpty()
        val viewModel: PreviewViewModel =
            viewModel(
                key = "server/$url/$processId",
                factory =
                    PreviewViewModel.factory(
                        PreviewTarget.Server(url, processId),
                        files,
                    ) { false },
            )
        val statuses by ServerProcessRegistry.statuses.collectAsStateWithLifecycle()
        val running = statuses[processId] == ServerStatus.Running
        LaunchedEffect(viewModel, running) { viewModel.setServerRunning(running) }
        PreviewPage(
            viewModel = viewModel,
            title = null,
            nav = nav,
            messages = messages,
            banner = stringResource(R.string.preview_server_banner, url),
            serverStopped = !running,
        )
    }
}

/** The preview screen for one view model. A null [title] shows the preview title. */
@Composable
private fun PreviewPage(
    viewModel: PreviewViewModel,
    title: String?,
    nav: TetherNav,
    messages: AppMessages,
    banner: String? = null,
    serverStopped: Boolean = false,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val linkNotOpened = stringResource(R.string.preview_link_blocked)
    PreviewScreen(
        state = state,
        title = title,
        onBack = nav::back,
        onReload = viewModel::reload,
        onNetworkChange = viewModel::setNetworkAllowed,
        banner = banner,
        serverStopped = serverStopped,
        page = { load ->
            PreviewWebView(
                load = load,
                networkAllowed = viewModel::networkAllowed,
                serverOrigin = state.serverOrigin,
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
