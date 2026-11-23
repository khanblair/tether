package app.tether.nav

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.R
import app.tether.di.AppModule
import app.tether.projectarchive.ContentResolverArchiveFiles
import app.tether.projectarchive.ExportProjectSheet
import app.tether.projectarchive.ExportProjectViewModel
import app.tether.projectarchive.ImportProjectScreen
import app.tether.projectarchive.ImportProjectViewModel
import app.tether.projectarchive.toArchiveUri
import java.util.UUID

private val ZIP_TYPES = arrayOf("application/zip", "application/x-zip-compressed")

/** The Import a ZIP screen. A finished import replaces it with the new project's home. */
internal fun NavGraphBuilder.projectArchiveRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(Route.ImportProject.path) {
        val context = LocalContext.current.applicationContext
        val import: ImportProjectViewModel =
            viewModel(
                factory =
                    viewModelFactory {
                        initializer {
                            ImportProjectViewModel(
                                module.projectArchiveGateway,
                                ContentResolverArchiveFiles(context),
                            )
                        }
                    },
            )
        val picker =
            rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
                if (uri == null) import.onPickCancelled() else import.onPicked(uri.toArchiveUri())
            }
        val state by import.state.collectAsStateWithLifecycle()
        ImportProjectScreen(
            state = state,
            onBack = nav::back,
            onChooseFile = { picker.launch(ZIP_TYPES) },
            onNameChange = import::onNameChange,
            onImport = import::onImport,
            onCancel = import::onCancel,
            onImported = { id -> nav.replace(projectHomePath(id), Route.ImportProject) },
        )
    }
}

/**
 * The Export project sheet. The system file picker chooses the destination, so the ZIP never
 * reaches a place the person did not pick. [onDismiss] closes the sheet.
 */
@Composable
internal fun ExportProjectHost(
    module: AppModule,
    projectId: String,
    projectName: String,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current.applicationContext
    val key = remember { "export-$projectId-${UUID.randomUUID()}" }
    val export: ExportProjectViewModel =
        viewModel(
            key = key,
            factory =
                viewModelFactory {
                    initializer {
                        ExportProjectViewModel(
                            projectId,
                            projectName,
                            module.projectArchiveGateway,
                            ContentResolverArchiveFiles(context),
                        )
                    }
                },
        )
    val picker =
        rememberLauncherForActivityResult(
            ActivityResultContracts.CreateDocument("application/zip"),
        ) { uri -> uri?.let { export.onDestinationChosen(it.toArchiveUri()) } }
    val state by export.state.collectAsStateWithLifecycle()
    val done = stringResource(R.string.archive_export_done)
    ExportProjectSheet(
        state = state,
        onIncludeIgnoredChange = export::onIncludeIgnoredChange,
        onChooseDestination = { suggested -> picker.launch(suggested) },
        onTryAgain = export::onTryAgain,
        onExported = {
            module.messages.show(done)
            onDismiss()
        },
        onDismiss = onDismiss,
    )
}
