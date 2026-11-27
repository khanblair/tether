package app.tether.workspace

import android.content.Context
import android.os.Environment
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Density
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherTextButton
import app.tether.settings.SettingsPage
import app.tether.settings.selectedTrailing

/** The Settings screen for the folder used by new projects. */
@Composable
fun WorkspaceScreen(
    viewModel: WorkspaceViewModel,
    capabilities: WorkspaceCapabilities,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    WorkspacePermissionEffect(viewModel, capabilities)
    SettingsPage(title = stringResource(R.string.workspace_title), onBack = onBack) {
        WorkspaceContent(
            state = state,
            sharedFolderAllowed = capabilities.sharedFolderAllowed,
            showCurrentFolder = true,
            showDataPaths = false,
            onChoose = viewModel::choose,
            onPathChange = viewModel::onPathChange,
            onSave = viewModel::save,
            onUsePrivate = viewModel::usePrivateStorage,
            onAllowAccess = { capabilities.openAllFilesAccessSettings(it) },
        )
    }
}

/** Shared workspace controls used by Settings and the onboarding step. */
@Composable
internal fun WorkspaceContent(
    state: WorkspaceUiState,
    sharedFolderAllowed: Boolean,
    showCurrentFolder: Boolean,
    showDataPaths: Boolean,
    onChoose: (WorkspaceChoice) -> Unit,
    onPathChange: (String) -> Unit,
    onSave: (() -> Unit) -> Unit,
    onUsePrivate: (() -> Unit) -> Unit,
    onAllowAccess: (Context) -> Unit,
) {
    if (showCurrentFolder) CurrentFolder(state)
    if (showDataPaths) WorkspaceDataList(state)
    if (!showCurrentFolder && !showDataPaths) {
        Text(
            text = stringResource(R.string.workspace_onboarding_intro),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
    }
    SectionTitle(text = stringResource(R.string.workspace_title))
    WorkspaceChoices(
        state = state,
        sharedFolderAllowed = sharedFolderAllowed,
        onChoose = onChoose,
    )
    Text(
        text = stringResource(R.string.workspace_new_projects_note),
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text3,
    )
    if (state.choice == WorkspaceChoice.Custom) {
        WorkspacePathField(state, onPathChange)
        WorkspaceProblemText(state.problem)
        Text(
            text = stringResource(R.string.workspace_custom_note),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
    }
    if (state.choice == WorkspaceChoice.Private) {
        WorkspaceProblemText(
            state.problem
                ?: if (!state.loading && state.location == null) {
                    WorkspaceProblem.Unavailable
                } else {
                    null
                },
        )
    }
    WorkspaceSaveAction(
        state = state,
        onSave = onSave,
        onUsePrivate = onUsePrivate,
        onAllowAccess = onAllowAccess,
    )
}

@Composable
private fun CurrentFolder(state: WorkspaceUiState) {
    state.location?.let { location ->
        SectionTitle(text = stringResource(R.string.workspace_current_folder))
        ListGroup {
            ListRow(
                title = location.effectiveRoot,
                subtitle =
                    stringResource(
                        if (location.newProjectRoot == null) {
                            R.string.workspace_private
                        } else {
                            R.string.workspace_custom
                        },
                    ),
            )
        }
    }
}

@Composable
private fun WorkspaceChoices(
    state: WorkspaceUiState,
    sharedFolderAllowed: Boolean,
    onChoose: (WorkspaceChoice) -> Unit,
) {
    ListGroup {
        ListRow(
            title = stringResource(R.string.workspace_private_choice),
            subtitle = stringResource(R.string.workspace_private_choice_body),
            trailing = { selectedTrailing(state.choice == WorkspaceChoice.Private) },
            onClick = { onChoose(WorkspaceChoice.Private) },
        )
        if (sharedFolderAllowed) {
            ListRow(
                title = stringResource(R.string.workspace_custom_choice),
                subtitle = stringResource(R.string.workspace_custom_choice_body),
                trailing = { selectedTrailing(state.choice == WorkspaceChoice.Custom) },
                onClick = { onChoose(WorkspaceChoice.Custom) },
            )
        }
    }
}

@Composable
private fun WorkspacePathField(
    state: WorkspaceUiState,
    onPathChange: (String) -> Unit,
) {
    OutlinedTextField(
        value = state.pathInput,
        onValueChange = onPathChange,
        modifier = Modifier.fillMaxWidth(),
        singleLine = true,
        isError = state.problem != null,
        label = { Text(text = stringResource(R.string.workspace_path_label)) },
        placeholder = { Text(text = stringResource(R.string.workspace_path_hint)) },
    )
}

@Composable
private fun WorkspaceProblemText(problem: WorkspaceProblem?) {
    problem?.let {
        Text(
            text = stringResource(workspaceProblemRes(it)),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.danger,
        )
    }
}

@Composable
private fun WorkspaceSaveAction(
    state: WorkspaceUiState,
    onSave: (() -> Unit) -> Unit,
    onUsePrivate: (() -> Unit) -> Unit,
    onAllowAccess: (Context) -> Unit,
) {
    val context = LocalContext.current
    when {
        state.choice == WorkspaceChoice.Private ->
            PrimaryButton(
                label = stringResource(R.string.workspace_use_private),
                onClick = { onUsePrivate {} },
                modifier = Modifier.fillMaxWidth(),
                loading = state.saving,
            )
        state.permissionRequired || !state.allFilesAccessGranted -> {
            PrimaryButton(
                label = stringResource(R.string.workspace_allow_access),
                onClick = { onAllowAccess(context) },
                modifier = Modifier.fillMaxWidth(),
            )
            TetherTextButton(
                label = stringResource(R.string.workspace_use_private),
                onClick = { onUsePrivate {} },
                enabled = !state.saving,
            )
        }
        else ->
            PrimaryButton(
                label = stringResource(R.string.workspace_save),
                onClick = { onSave {} },
                modifier = Modifier.fillMaxWidth(),
                loading = state.saving,
            )
    }
    if (state.permissionRequired) {
        Text(
            text = stringResource(R.string.workspace_permission_note),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
    }
}

@Composable
private fun WorkspaceDataList(state: WorkspaceUiState) {
    val paths = state.dataPaths
    SectionTitle(text = stringResource(R.string.workspace_current_folder))
    Column {
        DataPath(state.location?.defaultRoot ?: paths.projects, R.string.workspace_data_projects)
        DataPath(paths.database, R.string.workspace_data_database)
        DataPath(paths.secrets, R.string.workspace_data_secrets)
        DataPath(paths.checkpoints, R.string.workspace_data_checkpoints)
        DataPath(paths.memory, R.string.workspace_data_memory)
        DataPath(paths.skills, R.string.workspace_data_skills)
        DataPath(paths.vault, R.string.workspace_data_vault)
        DataPath(paths.crashReport, R.string.workspace_data_crash_report)
    }
}

@Composable
private fun DataPath(
    path: String,
    labelRes: Int,
) {
    if (path.isNotEmpty()) {
        Text(
            text = stringResource(labelRes) + ": " + path,
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
        )
    }
}

/** Keeps all-files access state current after the system settings screen closes. */
@Composable
internal fun WorkspacePermissionEffect(
    viewModel: WorkspaceViewModel,
    capabilities: WorkspaceCapabilities,
) {
    val context = LocalContext.current
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        viewModel.onAllFilesAccessChanged(capabilities.allFilesAccessGranted(context))
    }
}

/** Computes the shared-folder suggestion from Android's external storage directory. */
@Suppress("DEPRECATION")
fun suggestedWorkspacePath(): String =
    suggestedWorkspaceRoot(Environment.getExternalStorageDirectory().absolutePath)

private fun workspaceProblemRes(problem: WorkspaceProblem): Int =
    when (problem) {
        WorkspaceProblem.EmptyPath -> R.string.workspace_error_empty_path
        WorkspaceProblem.RootNotAbsolute -> R.string.workspace_error_not_absolute
        WorkspaceProblem.RootCannotBeCreated -> R.string.workspace_error_cannot_create
        WorkspaceProblem.RootNotDirectory -> R.string.workspace_error_not_directory
        WorkspaceProblem.RootIsSymlink -> R.string.workspace_error_symlink
        WorkspaceProblem.RootNotWritable -> R.string.workspace_error_not_writable
        WorkspaceProblem.RootInsidePrivateRoot -> R.string.workspace_error_inside_private
        WorkspaceProblem.RootIsFilesystemRoot -> R.string.workspace_error_filesystem_root
        WorkspaceProblem.RootInsideProject -> R.string.workspace_error_inside_project
        WorkspaceProblem.RootUnavailable -> R.string.workspace_error_unavailable
        WorkspaceProblem.Unavailable -> R.string.workspace_error_read
    }

@Preview(name = "Workspace, light", showBackground = true)
@Composable
internal fun WorkspaceLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { WorkspacePreview(previewState()) }
}

@Preview(name = "Workspace, dark", showBackground = true)
@Composable
internal fun WorkspaceDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        WorkspacePreview(
            state =
                WorkspaceUiState(
                    location =
                        WorkspaceLocation(
                            "/data/user/0/app/files/workspaces",
                            "/sdcard/Tether",
                            "/sdcard/Tether",
                        ),
                    choice = WorkspaceChoice.Custom,
                    pathInput = "/sdcard/Tether",
                    allFilesAccessGranted = true,
                ),
        )
    }
}

@Preview(name = "Workspace, 200% text", showBackground = true, fontScale = 2f)
@Composable
internal fun WorkspaceLargeTextPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        CompositionLocalProvider(LocalDensity provides Density(1f, 2f)) {
            WorkspacePreview(previewState())
        }
    }
}

@Preview(name = "Workspace, narrow", showBackground = true, widthDp = 320)
@Composable
internal fun WorkspaceNarrowPreview() {
    TetherTheme(mode = ThemeMode.Light) { WorkspacePreview(previewState()) }
}

@Composable
private fun WorkspacePreview(state: WorkspaceUiState) {
    SettingsPage(title = stringResource(R.string.workspace_title), onBack = {}) {
        WorkspaceContent(
            state = state,
            sharedFolderAllowed = true,
            showCurrentFolder = true,
            showDataPaths = false,
            onChoose = {},
            onPathChange = {},
            onSave = { _ -> },
            onUsePrivate = { _ -> },
            onAllowAccess = { _ -> },
        )
    }
}

private fun previewState() =
    WorkspaceUiState(
        location =
            WorkspaceLocation(
                "/data/user/0/app/files/workspaces",
                null,
                "/data/user/0/app/files/workspaces",
            ),
    )
