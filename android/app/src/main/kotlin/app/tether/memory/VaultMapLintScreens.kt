package app.tether.memory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/** The vault's Map tab: the core's text rendering of the notes around the start note. */
@Composable
fun VaultMapScreen(
    viewModel: VaultMapViewModel,
    onOpenTab: (VaultTab) -> Unit,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    VaultPage(
        title = stringResource(R.string.vault_map_title),
        selected = VaultTab.Map,
        onOpenTab,
        onBack,
    ) {
        state.problem?.let {
            InlineBanner(text = stringResource(it.messageRes()), tone = BannerTone.Danger)
        }
        if (state.loading) return@VaultPage
        if (state.lines.isEmpty()) {
            Text(
                text = stringResource(R.string.vault_map_empty),
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text2,
            )
        } else {
            MapLines(lines = state.lines)
        }
    }
}

@Composable
private fun MapLines(lines: List<String>) {
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(TetherRadius.md))
                .background(TetherTheme.colors.surfaceSunken)
                .padding(TetherSpace.space3),
    ) {
        lines.forEach { line ->
            Text(text = line, style = TetherTheme.monoSmall, color = TetherTheme.colors.text)
        }
    }
}

/** The vault's Lint tab. Only orphan notes come from the core, so the other checks say so. */
@Composable
fun VaultLintScreen(
    viewModel: VaultLintViewModel,
    onOpen: (String) -> Unit,
    onOpenTab: (VaultTab) -> Unit,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    VaultPage(
        title = stringResource(R.string.vault_lint_title),
        selected = VaultTab.Lint,
        onOpenTab,
        onBack,
    ) {
        state.problem?.let {
            InlineBanner(text = stringResource(it.messageRes()), tone = BannerTone.Danger)
        }
        if (state.loading) return@VaultPage
        SectionTitle(text = stringResource(R.string.vault_lint_orphans, state.orphans.size))
        if (state.orphans.isEmpty()) {
            Text(
                text = stringResource(R.string.vault_lint_none),
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text2,
            )
        } else {
            OrphanGroup(paths = state.orphans, onOpen = onOpen)
        }
        SectionTitle(text = stringResource(R.string.vault_lint_not_checked_title))
        Text(
            text = stringResource(R.string.vault_lint_not_checked),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
    }
}

@Composable
private fun OrphanGroup(
    paths: List<String>,
    onOpen: (String) -> Unit,
) {
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(shape)
                .background(TetherTheme.colors.surface)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape),
    ) {
        paths.forEachIndexed { index, path ->
            if (index > 0) {
                HorizontalDivider(
                    color = TetherTheme.colors.line,
                    thickness = TetherBorder.hairline,
                )
            }
            ListRow(title = path, onClick = { onOpen(path) })
        }
    }
}

/** The frame the Map and Lint tabs share: a top bar, the tabs, then the content. */
@Composable
private fun VaultPage(
    title: String,
    selected: VaultTab,
    onOpenTab: (VaultTab) -> Unit,
    onBack: () -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = { TetherTopBar(title = title, navigationIcon = { BackButton(onBack) }) },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(TetherSpace.space4),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Column(
                modifier = Modifier.widthIn(max = TetherSize.transcriptMax).fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
            ) {
                VaultTabs(selected = selected, onSelect = onOpenTab)
                content()
            }
        }
    }
}
