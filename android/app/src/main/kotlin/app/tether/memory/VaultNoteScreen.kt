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
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withLink
import androidx.compose.ui.text.withStyle
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.memory.BodyPart.BodyLink
import app.tether.memory.BodyPart.BodyText
import app.tether.shell.BackButton

/**
 * One note: its description, its body with links, and the notes that link to it. The note is read
 * again each time the screen resumes, so an edit shows at once.
 */
@Composable
fun VaultNoteScreen(
    viewModel: VaultNoteViewModel,
    onOpenNote: (String) -> Unit,
    onEdit: () -> Unit,
    onMap: () -> Unit,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    RefreshOnResume(onResume = viewModel::load)
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = state.note?.title ?: stringResource(R.string.vault_note_title),
                navigationIcon = { BackButton(onBack) },
                actions = {
                    if (state.note != null) {
                        TetherTextButton(
                            label = stringResource(R.string.vault_note_edit),
                            onClick = onEdit,
                        )
                    }
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
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
            ) {
                NoteContent(
                    state = state,
                    onOpenNote = onOpenNote,
                    onMap = onMap,
                    openLink = viewModel::openLink,
                )
            }
        }
    }
}

@Composable
private fun NoteContent(
    state: VaultNoteUiState,
    onOpenNote: (String) -> Unit,
    onMap: () -> Unit,
    openLink: (BodyLink, (String) -> Unit) -> Unit,
) {
    state.problem?.let {
        InlineBanner(text = stringResource(it.messageRes()), tone = BannerTone.Danger)
    }
    if (state.linkFailed) {
        InlineBanner(
            text = stringResource(R.string.vault_link_open_failed),
            tone = BannerTone.Danger,
        )
    }
    val note = state.note ?: return
    NoteHeader(description = note.description, type = note.noteType, tags = note.tags)
    NoteBody(parts = state.parts, onLink = { openLink(it, onOpenNote) })
    TetherTextButton(label = stringResource(R.string.vault_note_map), onClick = onMap)
    if (state.backlinks.isNotEmpty()) {
        SectionTitle(text = stringResource(R.string.vault_backlinks_title, state.backlinks.size))
        BacklinkGroup(paths = state.backlinks.map { it.from }, onOpen = onOpenNote)
    }
}

@Composable
private fun NoteHeader(
    description: String,
    type: String?,
    tags: List<String>,
) {
    val details = listOfNotNull(type, tags.takeIf { it.isNotEmpty() }?.joinToString(" ") { "#$it" })
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(TetherRadius.md))
                .background(TetherTheme.colors.surface)
                .border(
                    TetherBorder.hairline,
                    TetherTheme.colors.line,
                    RoundedCornerShape(TetherRadius.md),
                ).padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        Text(
            text = description,
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text,
        )
        details.forEach {
            Text(
                text = it,
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
    }
}

/** The body as one block of text. A resolved link is accent text that opens its note. */
@Composable
private fun NoteBody(
    parts: List<BodyPart>,
    onLink: (BodyLink) -> Unit,
) {
    val missingFormat = stringResource(R.string.vault_link_missing)
    val linkColor = TetherTheme.colors.accentInk
    val missingColor = TetherTheme.colors.text3
    val text: AnnotatedString =
        buildAnnotatedString {
            parts.forEachIndexed { index, part ->
                when (part) {
                    is BodyText -> append(part.text)
                    is BodyLink ->
                        if (part.resolved) {
                            val link =
                                LinkAnnotation.Clickable(
                                    tag = index.toString(),
                                    styles =
                                        TextLinkStyles(
                                            SpanStyle(
                                                color = linkColor,
                                                fontWeight = FontWeight.Medium,
                                            ),
                                        ),
                                    linkInteractionListener = { onLink(part) },
                                )
                            withLink(link) { append(part.label) }
                        } else {
                            withStyle(
                                SpanStyle(color = missingColor),
                            ) { append(missingFormat.format(part.label)) }
                        }
                }
            }
        }
    Text(text = text, style = MaterialTheme.typography.bodyLarge, color = TetherTheme.colors.text)
}

@Composable
private fun BacklinkGroup(
    paths: List<String>,
    onOpen: (String) -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(TetherRadius.md))
                .background(TetherTheme.colors.surface)
                .border(
                    TetherBorder.hairline,
                    TetherTheme.colors.line,
                    RoundedCornerShape(TetherRadius.md),
                ),
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

/** Runs [onResume] each time the screen resumes, so a note edited above it is read again. */
@Composable
private fun RefreshOnResume(onResume: () -> Unit) {
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner) {
        val observer =
            LifecycleEventObserver { _, event ->
                if (event == Lifecycle.Event.ON_RESUME) onResume()
            }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
}
