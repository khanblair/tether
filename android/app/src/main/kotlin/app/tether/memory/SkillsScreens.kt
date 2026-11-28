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
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherSearchField
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/** The skill list: a search field, then one row per skill. An invalid skill shows its error. */
@Composable
fun SkillsScreen(
    viewModel: SkillsViewModel,
    onOpen: (String) -> Unit,
    onInstall: () -> Unit,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { viewModel.refresh() }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.skills_title),
                navigationIcon = { BackButton(onBack) },
                actions = {
                    TetherTextButton(
                        label = stringResource(R.string.skills_install),
                        onClick = onInstall,
                    )
                },
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
                SkillsContent(
                    state = state,
                    onQueryChange = viewModel::onQueryChange,
                    onOpen = onOpen,
                )
            }
        }
    }
}

@Composable
private fun SkillsContent(
    state: SkillsUiState,
    onQueryChange: (String) -> Unit,
    onOpen: (String) -> Unit,
) {
    state.problem?.let {
        InlineBanner(text = stringResource(it.messageRes()), tone = BannerTone.Danger)
    }
    if (state.loading) return
    if (state.total == 0 && state.problem == null) {
        Text(
            text = stringResource(R.string.skills_empty),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
        return
    }
    TetherSearchField(
        value = state.query,
        onValueChange = onQueryChange,
        placeholder = stringResource(R.string.skills_search_placeholder),
        clearLabel = stringResource(R.string.skills_search_clear),
    )
    if (state.skills.isEmpty()) {
        Text(
            text = stringResource(R.string.skills_no_match),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
    } else {
        SkillGroup(skills = state.skills, onOpen = onOpen)
    }
}

@Composable
private fun SkillGroup(
    skills: List<SkillItem>,
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
        skills.forEachIndexed { index, skill ->
            if (index > 0) {
                HorizontalDivider(
                    color = TetherTheme.colors.line,
                    thickness = TetherBorder.hairline,
                )
            }
            SkillRow(skill = skill, onOpen = onOpen)
        }
    }
}

@Composable
private fun SkillRow(
    skill: SkillItem,
    onOpen: (String) -> Unit,
) {
    if (skill.valid) {
        ListRow(
            title = skill.name,
            subtitle = skill.description,
            onClick = { onOpen(skill.folder) },
        )
    } else {
        ListRow(
            title = skill.folder,
            subtitle = stringResource(R.string.skills_invalid_prefix, skill.error.orEmpty()),
        )
    }
}

/** One skill: its parsed fields and its body as text. Read only. */
@Composable
fun SkillDetailScreen(
    viewModel: SkillDetailViewModel,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val detail = state.detail
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = detail?.name ?: stringResource(R.string.skills_detail_title),
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
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            state.problem?.let {
                InlineBanner(text = stringResource(it.messageRes()), tone = BannerTone.Danger)
            }
            detail?.let { SkillFields(it) }
        }
    }
}

@Composable
private fun SkillFields(detail: SkillDetail) {
    Text(
        text = detail.description,
        style = MaterialTheme.typography.bodyLarge,
        color = TetherTheme.colors.text,
    )
    val fields =
        listOfNotNull(
            detail.license?.let { stringResource(R.string.skills_field_license) to it },
            detail.compatibility?.let { stringResource(R.string.skills_field_compatibility) to it },
            detail.allowedTools?.let { stringResource(R.string.skills_field_tools) to it },
        ) + detail.metadata.map { (key, value) -> key to value }
    fields.forEach { (label, value) -> FieldLine(label = label, value = value) }
    SectionTitle(text = stringResource(R.string.skills_body_title))
    Text(
        text = detail.body,
        style = TetherTheme.mono,
        color = TetherTheme.colors.text,
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(TetherRadius.md))
                .background(TetherTheme.colors.surfaceSunken)
                .padding(TetherSpace.space3),
    )
}

@Composable
private fun FieldLine(
    label: String,
    value: String,
) {
    Column {
        Text(
            text = label,
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text,
        )
    }
}
