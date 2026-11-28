package app.tether.memory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListRow
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/**
 * The sample gallery: the bundled sample skills, each with an Install button, then the sources
 * and licenses. Reached from the Skills list's Install button.
 */
@Composable
fun SampleSkillsScreen(
    viewModel: SampleSkillsViewModel,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.skill_samples_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
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
                SampleSkillsContent(state = state, onInstall = viewModel::install)
            }
        }
    }
}

@Composable
private fun SampleSkillsContent(
    state: SampleSkillsUiState,
    onInstall: (String) -> Unit,
) {
    state.problem?.let {
        InlineBanner(text = stringResource(it.messageRes()), tone = BannerTone.Danger)
    }
    Text(
        text = stringResource(R.string.skill_samples_intro),
        style = MaterialTheme.typography.bodyMedium,
        color = TetherTheme.colors.text2,
    )
    if (state.loading) return
    SampleGroup(state = state, onInstall = onInstall)
    state.notice?.let { SourcesBlock(notice = it) }
}

@Composable
private fun SampleGroup(
    state: SampleSkillsUiState,
    onInstall: (String) -> Unit,
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
        state.items.forEachIndexed { index, item ->
            if (index > 0) {
                HorizontalDivider(
                    color = TetherTheme.colors.line,
                    thickness = TetherBorder.hairline,
                )
            }
            SampleRow(
                item = item,
                installing = item.name in state.installing,
                onInstall = onInstall,
            )
        }
    }
}

@Composable
private fun SampleRow(
    item: SampleSkillItem,
    installing: Boolean,
    onInstall: (String) -> Unit,
) {
    ListRow(
        title = item.name,
        subtitle = item.description,
        subtitleMaxLines = SAMPLE_DESCRIPTION_LINES,
        trailing = {
            SampleTrailing(item = item, installing = installing, onInstall = onInstall)
        },
    )
}

@Composable
private fun SampleTrailing(
    item: SampleSkillItem,
    installing: Boolean,
    onInstall: (String) -> Unit,
) {
    when {
        item.installed ->
            Text(
                text = stringResource(R.string.skill_samples_installed),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text3,
            )
        installing ->
            Text(
                text = stringResource(R.string.skill_samples_installing),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.textDisabled,
            )
        else -> {
            val label = stringResource(R.string.skill_samples_install_named, item.name)
            TetherTextButton(
                label = stringResource(R.string.skill_samples_install),
                onClick = { onInstall(item.name) },
                modifier = Modifier.semantics { contentDescription = label },
            )
        }
    }
}

/** A sample description shows at most this many lines. */
private const val SAMPLE_DESCRIPTION_LINES = 3

/** The sources and licenses row. It opens to show the notice text, which can be selected. */
@Composable
private fun SourcesBlock(notice: String) {
    var expanded by rememberSaveable { mutableStateOf(false) }
    val shape = RoundedCornerShape(TetherRadius.md)
    val label = stringResource(R.string.skill_samples_sources_description)
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(shape)
                .background(TetherTheme.colors.surface)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape),
    ) {
        ListRow(
            title = stringResource(R.string.skill_samples_sources),
            onClick = { expanded = !expanded },
            trailing = {
                TetherIcon(
                    icon = if (expanded) TetherIcons.ChevronDown else TetherIcons.ChevronRight,
                    contentDescription = null,
                    tint = TetherTheme.colors.text2,
                    size = TetherSize.iconXs,
                )
            },
            modifier = Modifier.semantics { contentDescription = label },
        )
        if (expanded) {
            HorizontalDivider(
                color = TetherTheme.colors.line,
                thickness = TetherBorder.hairline,
            )
            SelectionContainer {
                Text(
                    text = notice,
                    style = TetherTheme.monoSmall,
                    color = TetherTheme.colors.text2,
                    modifier = Modifier.padding(TetherSpace.space4),
                )
            }
        }
    }
}
