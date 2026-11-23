@file:Suppress("MagicNumber")

package app.tether.settings.executors

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.ListGroup
import app.tether.design.tokens.TetherSpace

private const val SAMPLE_MB = 1_000_000L

@Preview(name = "Runtime packs, light", showBackground = true)
@Composable
internal fun RuntimePackRowLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { RuntimePackSample() }
}

@Preview(name = "Runtime packs, dark", showBackground = true)
@Composable
internal fun RuntimePackRowDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { RuntimePackSample() }
}

@Composable
private fun RuntimePackSample() {
    ListGroup {
        RuntimePackRow(
            pack = sampleRow("python-3.12", PackState.NotInstalled, size = 40 * SAMPLE_MB),
            onInstall = {},
            onCancel = {},
        )
        RuntimePackRow(
            pack = sampleRow("quickjs", PackState.Downloading(12 * SAMPLE_MB, 40 * SAMPLE_MB)),
            onInstall = {},
            onCancel = {},
        )
        RuntimePackRow(
            pack = sampleRow("sqlite", PackState.Verifying),
            onInstall = {},
            onCancel = {},
        )
        RuntimePackRow(
            pack = sampleRow("duckdb", PackState.Detached),
            onInstall = {},
            onCancel = {},
        )
        RuntimePackRow(
            pack = sampleRow("node-22", PackState.Ready("22.4.0"), size = 96 * SAMPLE_MB),
            onInstall = {},
            onCancel = {},
        )
        RuntimePackRow(
            pack = sampleRow("go-1.24", PackState.Failed(ExecutorsFailure.VerificationFailed)),
            onInstall = {},
            onCancel = {},
        )
        RuntimePackRow(
            pack = sampleRow("rust-1.90", PackState.NotAvailable),
            onInstall = {},
            onCancel = {},
        )
    }
}

@Preview(name = "Linux mode, light", showBackground = true)
@Composable
internal fun LinuxSetupCardLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { LinuxSample() }
}

@Preview(name = "Linux mode, dark", showBackground = true)
@Composable
internal fun LinuxSetupCardDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { LinuxSample() }
}

@Composable
private fun LinuxSample() {
    ListGroup {
        LinuxSetupCard(state = LinuxState.NotInstalled, onInstall = {}, onRepair = null)
        LinuxSetupCard(state = LinuxState.SettingUp(0L, 0L), onInstall = {}, onRepair = null)
        LinuxSetupCard(
            state = LinuxState.SettingUp(30 * SAMPLE_MB, 120 * SAMPLE_MB),
            onInstall = {},
            onRepair = null,
        )
        LinuxSetupCard(
            state = LinuxState.SettingUp(120 * SAMPLE_MB, 120 * SAMPLE_MB),
            onInstall = {},
            onRepair = null,
        )
        LinuxSetupCard(state = LinuxState.Ready, onInstall = {}, onRepair = {})
        LinuxSetupCard(
            state = LinuxState.Broken(ExecutorsFailure.Unknown),
            onInstall = {},
            onRepair = {},
        )
        LinuxSetupCard(
            state = LinuxState.Unavailable(LinuxUnavailable.ThisBuild),
            onInstall = {},
            onRepair = null,
        )
    }
}

@Preview(name = "CI runs, light", showBackground = true)
@Composable
internal fun CiRunsListLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { CiRunsSample() }
}

@Preview(name = "CI runs, dark", showBackground = true)
@Composable
internal fun CiRunsListDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { CiRunsSample() }
}

@Composable
private fun CiRunsSample() {
    Column(
        modifier = Modifier.fillMaxWidth().padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
    ) {
        CiRunsList(runs = null, onOpen = {})
        CiRunsList(runs = emptyList(), onOpen = {})
        CiRunsList(
            runs =
                listOf(
                    sampleRun(1, "Fix login timeout", CiRunState.Succeeded, 64),
                    sampleRun(2, "Add token refresh", CiRunState.Failed, 248),
                    sampleRun(3, "Bump lint rules", CiRunState.Running, 5),
                    sampleRun(4, "Update docs", CiRunState.Queued, 0),
                    sampleRun(5, "Remove cache", CiRunState.Cancelled, 12),
                ),
            onOpen = {},
        )
    }
}

private fun sampleRow(
    id: String,
    state: PackState,
    size: Long = 0L,
): PackRow = PackRow(id = id, version = "1.0.0", sizeBytes = size, state = state)

private fun sampleRun(
    id: Long,
    title: String,
    state: CiRunState,
    seconds: Long,
): CiRunRow =
    CiRunRow(
        id = id,
        title = title,
        branch = "main",
        state = state,
        durationSeconds = seconds,
        startedAtMs = null,
        htmlUrl = "",
    )
