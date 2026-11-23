package app.tether.projectarchive

import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.home.ProjectNameProblem

private const val SAMPLE_BYTES = 4_404_019L
private const val LARGE_BYTES = 250L * 1024L * 1024L
private const val LONG_NAME_LENGTH = 81

@Preview(name = "Import, idle, light", showBackground = true)
@Composable
internal fun ImportIdleLightPreview() {
    ImportPreview(ThemeMode.Light, ImportProjectUiState())
}

@Preview(name = "Import, picked, dark", showBackground = true)
@Composable
internal fun ImportPickedDarkPreview() {
    ImportPreview(
        ThemeMode.Dark,
        ImportProjectUiState(
            phase = ImportPhase.Picked,
            fileName = "api-server.zip",
            sizeBytes = SAMPLE_BYTES,
            name = "api-server",
            nameUsable = true,
        ),
    )
}

@Preview(name = "Import, picked, name too long, light", showBackground = true)
@Composable
internal fun ImportNameTooLongPreview() {
    ImportPreview(
        ThemeMode.Light,
        ImportProjectUiState(
            phase = ImportPhase.Picked,
            fileName = "api-server.zip",
            sizeBytes = SAMPLE_BYTES,
            name = "a".repeat(LONG_NAME_LENGTH),
            nameProblem = ProjectNameProblem.TooLong,
        ),
    )
}

@Preview(name = "Import, file too large, light", showBackground = true)
@Composable
internal fun ImportTooLargePreview() {
    ImportPreview(
        ThemeMode.Light,
        ImportProjectUiState(
            phase = ImportPhase.Picked,
            fileName = "monorepo.zip",
            sizeBytes = LARGE_BYTES,
            tooLarge = true,
            name = "monorepo",
            nameUsable = true,
        ),
    )
}

@Preview(name = "Import, importing, dark", showBackground = true)
@Composable
internal fun ImportingDarkPreview() {
    ImportPreview(
        ThemeMode.Dark,
        ImportProjectUiState(
            phase = ImportPhase.Importing,
            fileName = "api-server.zip",
            name = "api-server",
            nameUsable = true,
        ),
    )
}

@Preview(name = "Import, failed, light", showBackground = true)
@Composable
internal fun ImportFailedPreview() {
    ImportPreview(
        ThemeMode.Light,
        ImportProjectUiState(
            phase = ImportPhase.Failed,
            fileName = "api-server.zip",
            sizeBytes = SAMPLE_BYTES,
            name = "api-server",
            nameUsable = true,
            reason = ArchiveReason.NotZip,
        ),
    )
}

@Preview(name = "Export, editing, light", showBackground = true)
@Composable
internal fun ExportEditingLightPreview() {
    ExportPreview(ThemeMode.Light, ExportProjectUiState(projectName = "acme-api"))
}

@Preview(name = "Export, exporting, dark", showBackground = true)
@Composable
internal fun ExportExportingDarkPreview() {
    ExportPreview(
        ThemeMode.Dark,
        ExportProjectUiState(projectName = "acme-api", phase = ExportPhase.Exporting),
    )
}

@Preview(name = "Export, failed, light", showBackground = true)
@Composable
internal fun ExportFailedLightPreview() {
    ExportPreview(
        ThemeMode.Light,
        ExportProjectUiState(
            projectName = "acme-api",
            includeIgnored = true,
            phase = ExportPhase.Failed,
            reason = ArchiveReason.WriteFailed,
        ),
    )
}

@Composable
private fun ImportPreview(
    mode: ThemeMode,
    state: ImportProjectUiState,
) {
    TetherTheme(mode = mode) {
        Surface(color = TetherTheme.colors.bg) {
            ImportProjectScreen(
                state = state,
                onBack = {},
                onChooseFile = {},
                onNameChange = {},
                onImport = {},
                onCancel = {},
                onImported = {},
            )
        }
    }
}

@Composable
private fun ExportPreview(
    mode: ThemeMode,
    state: ExportProjectUiState,
) {
    TetherTheme(mode = mode) {
        Surface(color = TetherTheme.colors.surface) {
            ExportProjectBody(
                state = state,
                onIncludeIgnoredChange = {},
                onChooseDestination = {},
                onTryAgain = {},
            )
        }
    }
}
