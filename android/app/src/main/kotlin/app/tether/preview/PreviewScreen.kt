package app.tether.preview

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import app.tether.R
import app.tether.design.RefreshCw
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.WifiOff
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.TetherChip
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.files.FilesProblem
import app.tether.shell.BackButton

/**
 * The preview screen. The title is the file name, or "Preview" when [title] is null. [page]
 * draws the WebView for a loaded page, and the caller rebuilds it when [PreviewUiState.reloadKey]
 * changes.
 */
@Composable
fun PreviewScreen(
    state: PreviewUiState,
    title: String?,
    onBack: () -> Unit,
    onReload: () -> Unit,
    onNetworkChange: (Boolean) -> Unit,
    page: @Composable (PreviewLoad) -> Unit,
    modifier: Modifier = Modifier,
    banner: String? = null,
    serverStopped: Boolean = false,
) {
    Scaffold(
        modifier = modifier,
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = title ?: stringResource(R.string.preview_title),
                navigationIcon = { BackButton(onBack) },
                actions = {
                    // An expired inline preview has no page to reload and no network to switch.
                    if (state.load != PreviewLoad.Expired) {
                        TetherIconButton(
                            icon = TetherIcons.RefreshCw,
                            contentDescription = stringResource(R.string.preview_reload),
                            onClick = onReload,
                        )
                        NetworkChip(
                            allowed = state.networkAllowed,
                            locked = state.networkLocked,
                            onToggle = { onNetworkChange(!state.networkAllowed) },
                        )
                    }
                },
            )
        },
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            banner?.let {
                InlineBanner(
                    text = it,
                    tone = BannerTone.Info,
                    modifier = Modifier.padding(horizontal = TetherSpace.space4),
                )
            }
            val blocked = state.blocked.size
            if (blocked > 0 && !state.networkAllowed && !state.networkLocked) {
                InlineBanner(
                    text = pluralStringResource(R.plurals.preview_blocked, blocked, blocked),
                    tone = BannerTone.Warning,
                    actionLabel = stringResource(R.string.preview_turn_on),
                    onAction = { onNetworkChange(true) },
                    modifier = Modifier.padding(horizontal = TetherSpace.space4),
                )
            }
            if (serverStopped || !state.serverRunning) {
                InlineBanner(
                    text = stringResource(R.string.preview_server_stopped),
                    tone = BannerTone.Warning,
                    actionLabel = stringResource(R.string.preview_server_reload),
                    onAction = onReload,
                    modifier = Modifier.padding(TetherSpace.space4),
                )
            } else {
                PreviewBody(
                    load = state.load,
                    reloadKey = state.reloadKey,
                    page = page,
                    onReload = onReload,
                )
            }
        }
    }
}

private fun networkLabel(allowed: Boolean): Int =
    if (allowed) R.string.preview_network_on else R.string.preview_network_off

/**
 * The network switch. Off is the default for each project. Accent is not used: it is a toggle. A
 * [locked] switch shows its state and takes no tap, as an inline preview has no network.
 */
@Composable
private fun NetworkChip(
    allowed: Boolean,
    locked: Boolean,
    onToggle: () -> Unit,
) {
    TetherChip(
        label = stringResource(networkLabel(allowed)),
        leadingIcon = if (allowed) TetherIcons.Globe else TetherIcons.WifiOff,
        onClick = if (locked) null else onToggle,
        modifier = Modifier.heightIn(min = TetherSize.touchMin).padding(end = TetherSpace.space2),
    )
}

@Composable
private fun ColumnScope.PreviewBody(
    load: PreviewLoad,
    reloadKey: Int,
    page: @Composable (PreviewLoad) -> Unit,
    onReload: () -> Unit,
) {
    val padding = Modifier.padding(TetherSpace.space4)
    when (load) {
        PreviewLoad.Loading ->
            Text(
                text = stringResource(R.string.preview_loading),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text3,
                modifier = padding,
            )
        is PreviewLoad.Page, is PreviewLoad.Html ->
            Box(modifier = Modifier.fillMaxWidth().weight(1f)) {
                key(reloadKey) { page(load) }
            }
        PreviewLoad.Missing ->
            Text(
                text = stringResource(R.string.preview_missing),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
                modifier = padding,
            )
        PreviewLoad.Expired ->
            Text(
                text = stringResource(R.string.preview_expired),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
                modifier = padding,
            )
        is PreviewLoad.Failed ->
            InlineBanner(
                text = stringResource(R.string.preview_failed),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.preview_retry),
                onAction = onReload,
                modifier = padding,
            )
    }
}

@Preview(name = "Preview, loaded, light", showBackground = true)
@Composable
internal fun PreviewLoadedLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        PreviewSample(PreviewUiState(load = PreviewLoad.Page("https://preview.tether.invalid/")))
    }
}

@Preview(name = "Preview, blocked banner, dark", showBackground = true)
@Composable
internal fun PreviewBlockedDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        PreviewSample(
            PreviewUiState(
                load = PreviewLoad.Page("https://preview.tether.invalid/"),
                blocked = setOf("https://example.com/a.js", "https://example.com/b.css"),
            ),
        )
    }
}

@Preview(name = "Preview, missing file", showBackground = true)
@Composable
internal fun PreviewMissingPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        PreviewSample(PreviewUiState(load = PreviewLoad.Missing))
    }
}

@Preview(name = "Preview, error", showBackground = true)
@Composable
internal fun PreviewErrorPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        PreviewSample(PreviewUiState(load = PreviewLoad.Failed(FilesProblem.Unavailable)))
    }
}

@Composable
private fun PreviewSample(state: PreviewUiState) {
    PreviewScreen(
        state = state,
        title = "index.html",
        onBack = {},
        onReload = {},
        onNetworkChange = {},
        page = { Box(Modifier.fillMaxSize().background(TetherTheme.colors.codeBg)) },
    )
}
