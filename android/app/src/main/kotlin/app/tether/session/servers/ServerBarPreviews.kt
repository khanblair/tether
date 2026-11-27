package app.tether.session.servers

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode

/** The dev server port in the samples. */
private const val DEV_PORT = 5173

/** The preview server port in the samples. */
private const val PREVIEW_PORT = 4173

@Preview(name = "One server, light", showBackground = true)
@Composable
internal fun OneServerLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        ServerBarSample(ServerState(listOf(process(DEV_PORT))), null)
    }
}

@Preview(name = "Two servers, dark", showBackground = true)
@Composable
internal fun TwoServersDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        ServerBarSample(ServerState(listOf(process(DEV_PORT), process(PREVIEW_PORT, "api"))), null)
    }
}

@Preview(
    name = "Stopped server, 200% font, 320 dp",
    widthDp = 320,
    fontScale = 2f,
    showBackground = true,
)
@Composable
internal fun StoppedServerPreview() {
    TetherTheme {
        ServerBarSample(
            ServerState(listOf(process(DEV_PORT).copy(status = ServerStatus.Stopped))),
            ServerPreviewTarget.Server("http://127.0.0.1:$DEV_PORT", "dev"),
        )
    }
}

@Composable
private fun ServerBarSample(
    state: ServerState,
    preview: ServerPreviewTarget?,
) {
    ServerBar(state, preview, onPreview = {}, onStop = {})
}

private fun process(
    port: Int,
    id: String = "dev",
) = ServerProcess(id, "npm run dev", ServerStatus.Running, port.toUShort())
