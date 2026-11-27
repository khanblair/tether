package app.tether.newsession

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherChip
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.home.ProjectPickerSheet
import app.tether.home.relativeTime
import app.tether.home.text
import app.tether.session.Composer
import app.tether.session.SessionMode
import app.tether.session.SessionStatus
import app.tether.session.StatusLine
import app.tether.shell.BackButton

/** The real New session screen. It creates nothing until the composer sends text. */
@Composable
fun NewSessionScreen(
    state: NewSessionUiState,
    onBack: () -> Unit,
    onSelectProject: (String) -> Unit,
    onSend: (String) -> Unit,
    onNewProject: () -> Unit,
    onRetry: () -> Unit,
) {
    var pickerOpen by rememberSaveable { mutableStateOf(false) }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            NewSessionTopBar(
                state = state,
                onBack = onBack,
                onOpenPicker = { pickerOpen = true },
            )
        },
        bottomBar = {
            if (state is NewSessionUiState.Ready) {
                NewSessionComposer(state = state, onSend = onSend)
            }
        },
    ) { padding ->
        NewSessionBody(
            state = state,
            onNewProject = onNewProject,
            onRetry = onRetry,
            modifier = Modifier.fillMaxSize().padding(padding),
        )
    }
    if (pickerOpen) {
        ProjectPickerSheet(
            projects = (state as? NewSessionUiState.Ready)?.projects.orEmpty(),
            selectedId = (state as? NewSessionUiState.Ready)?.project?.id,
            onSelect = { id ->
                pickerOpen = false
                onSelectProject(id)
            },
            onDismiss = { pickerOpen = false },
        )
    }
}

@Composable
private fun NewSessionTopBar(
    state: NewSessionUiState,
    onBack: () -> Unit,
    onOpenPicker: () -> Unit,
) {
    val project = (state as? NewSessionUiState.Ready)?.project
    TetherTopBar(
        title = stringResource(R.string.route_new_session_title),
        subtitle = project?.name,
        navigationIcon = { BackButton(onBack) },
        actions = {
            project?.let {
                TetherChip(
                    label = it.name,
                    trailingChevron = true,
                    onClick = onOpenPicker,
                )
            }
        },
    )
}

@Composable
private fun NewSessionBody(
    state: NewSessionUiState,
    onNewProject: () -> Unit,
    onRetry: () -> Unit,
    modifier: Modifier,
) {
    when (state) {
        NewSessionUiState.Loading -> Unit
        NewSessionUiState.NoProject ->
            EmptyProjectBody(onNewProject = onNewProject, modifier = modifier)
        NewSessionUiState.Failed ->
            InlineBanner(
                text = stringResource(R.string.new_session_load_failed),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.new_session_retry),
                onAction = onRetry,
                modifier = modifier.padding(TetherSpace.space4),
            )
        is NewSessionUiState.Ready ->
            ReadyBody(
                state = state,
                modifier = modifier,
            )
    }
}

@Composable
private fun EmptyProjectBody(
    onNewProject: () -> Unit,
    modifier: Modifier,
) {
    Column(
        modifier = modifier.padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Text(
            text = stringResource(R.string.new_session_no_project),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
        PrimaryButton(
            label = stringResource(R.string.new_session_new_project),
            onClick = onNewProject,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun ReadyBody(
    state: NewSessionUiState.Ready,
    modifier: Modifier,
) {
    LazyColumn(
        modifier = modifier,
        contentPadding =
            androidx.compose.foundation.layout
                .PaddingValues(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
    ) {
        item { LoadedCard(state.preview) }
        state.preview.lastSessionTitle?.let { title ->
            item {
                LastSessionLine(
                    title = title,
                    updatedAtMs = state.preview.lastSessionUpdatedAtMs,
                )
            }
        }
    }
}

@Composable
private fun LoadedCard(preview: NewSessionPreview) {
    Surface(
        color = TetherTheme.colors.surfaceSunken,
        shape = RoundedCornerShape(TetherRadius.md),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.padding(TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            Text(
                text = stringResource(R.string.new_session_loaded_title),
                style = MaterialTheme.typography.titleSmall,
                color = TetherTheme.colors.text,
            )
            ContextRow(
                icon = TetherIcons.FileText,
                title =
                    preview.instructionsFile
                        ?: stringResource(R.string.new_session_no_instructions),
                detail = instructionsLine(preview),
            )
            ContextRow(
                icon = TetherIcons.Brain,
                title = stringResource(R.string.new_session_memory_title),
                detail = memoryLine(preview.memoryEntries),
            )
            ContextRow(
                icon = TetherIcons.Package,
                title = stringResource(R.string.new_session_skills_title),
                detail = skillsLine(preview.skillsAvailable),
            )
        }
    }
}

@Composable
private fun ContextRow(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    title: String,
    detail: String,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TetherIcon(
            icon = icon,
            contentDescription = null,
            tint = TetherTheme.colors.text2,
            size = TetherSize.iconS,
        )
        Column(modifier = Modifier.weight(1f)) {
            Text(text = title, style = MaterialTheme.typography.bodyLarge)
            Text(
                text = detail,
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
    }
}

@Composable
private fun instructionsLine(preview: NewSessionPreview): String =
    if (instructionsDetail(preview.instructionsFile)) {
        pluralStringResource(
            R.plurals.new_session_instruction_lines,
            preview.instructionsLines,
            preview.instructionsLines,
        )
    } else {
        stringResource(R.string.new_session_instructions_helper)
    }

@Composable
private fun memoryLine(count: Int): String =
    when (detailCount(count)) {
        DetailCount.None -> stringResource(R.string.new_session_memory_none)
        DetailCount.One,
        DetailCount.Many,
        -> pluralStringResource(R.plurals.new_session_memory_entries, count, count)
    }

@Composable
private fun skillsLine(count: Int): String =
    when (detailCount(count)) {
        DetailCount.None -> stringResource(R.string.new_session_skills_none)
        DetailCount.One,
        DetailCount.Many,
        -> pluralStringResource(R.plurals.new_session_skills_available, count, count)
    }

@Composable
private fun LastSessionLine(
    title: String,
    updatedAtMs: Long?,
) {
    val time = updatedAtMs ?: return
    val now = remember { System.currentTimeMillis() }
    Text(
        text =
            stringResource(
                R.string.new_session_last_session,
                title,
                relativeTime(now, time).text(),
            ),
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text3,
        modifier = Modifier.fillMaxWidth(),
    )
}

@Composable
private fun NewSessionComposer(
    state: NewSessionUiState.Ready,
    onSend: (String) -> Unit,
) {
    val model = state.modelId ?: stringResource(R.string.new_session_model_unavailable)
    val status =
        SessionStatus(
            modelId = model,
            mode = state.mode.coreModeName(),
            tokensIn = 0L,
            tokensOut = 0L,
            costMicros = 0L,
            contextWindow = null,
        )
    Column(modifier = Modifier.background(TetherTheme.colors.bg)) {
        StatusLine(
            status = status,
            usage = null,
            modelName = model,
            onModelClick = null,
            onModeClick = null,
        )
        Composer(
            modelLabel = model,
            mode = state.mode,
            running = false,
            onSend = onSend,
            onStop = {},
            sendBlocked = state.starting,
        )
    }
}

private fun SessionMode.coreModeName(): String =
    when (this) {
        SessionMode.ACCEPT_EDITS -> "accept_edits"
        else -> name.lowercase()
    }
