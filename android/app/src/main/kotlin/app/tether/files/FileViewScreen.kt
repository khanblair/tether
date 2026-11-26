package app.tether.files

import android.text.format.Formatter
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace
import app.tether.messages.AppMessages
import app.tether.preview.isPreviewable
import app.tether.shell.BackButton

/** The width of the line-number gutter in the code view. */
private val gutterWidth: Dp
    get() = TetherSpace.space10

/**
 * The read-only file viewer. The name is the title, and the folder and line count are the
 * subtitle. Long lines scroll sideways, and the line numbers scroll with their lines.
 */
@Composable
fun FileViewScreen(
    viewModel: FileViewViewModel,
    path: String,
    toasts: AppMessages,
    onBack: () -> Unit,
    onPreview: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val copyPath = rememberCopyPath(toasts)
    val context = LocalContext.current
    val format: (Long) -> String = { Formatter.formatShortFileSize(context, it) }
    val preview = state.preview
    val lineCount = preview?.text?.let { state.lines.size }
    val folder = folderOf(path)
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = path.substringAfterLast('/'),
                subtitle = viewerSubtitle(folder, lineCount),
                navigationIcon = { BackButton(onBack) },
                actions = {
                    if (isPreviewable(path)) {
                        TetherIconButton(
                            icon = TetherIcons.Eye,
                            contentDescription = stringResource(R.string.files_preview_action),
                            onClick = onPreview,
                        )
                    }
                    TetherTextButton(
                        label = stringResource(R.string.files_copy_path),
                        onClick = { copyPath(path) },
                    )
                },
            )
        },
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            ViewerContent(
                state = state,
                preview = preview,
                format = format,
                onRetry = viewModel::retry,
            )
        }
    }
}

@Composable
private fun ViewerContent(
    state: FileViewUiState,
    preview: FilePreview?,
    format: (Long) -> String,
    onRetry: () -> Unit,
) {
    when {
        state.problem != null ->
            InlineBanner(
                text = stringResource(R.string.files_view_failed),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.files_try_again),
                onAction = onRetry,
                modifier = Modifier.padding(TetherSpace.space4),
            )
        state.loading ->
            ThinLoadingLine(
                label = stringResource(R.string.files_loading),
                modifier = Modifier.padding(TetherSpace.space4),
            )
        preview == null -> Unit
        preview.binary -> BinaryNotice(sizeText = format(preview.sizeBytes))
        else -> TextBody(preview = preview, lines = state.lines, format = format)
    }
}

/** The folder and line count, such as "src/auth · 112 lines". The count is null before a read. */
@Composable
private fun viewerSubtitle(
    folder: String,
    lineCount: Int?,
): String {
    val where = folder.ifEmpty { stringResource(R.string.files_view_root) }
    if (lineCount == null) return where
    val lines = pluralStringResource(R.plurals.files_view_line_count, lineCount, lineCount)
    return "$where · $lines"
}

@Composable
private fun BinaryNotice(sizeText: String) {
    Column(
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        modifier = Modifier.padding(TetherSpace.space4),
    ) {
        Text(
            text = stringResource(R.string.files_view_binary),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text,
        )
        Text(
            text = stringResource(R.string.files_view_size, sizeText),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
    }
}

@Composable
private fun TextBody(
    preview: FilePreview,
    lines: List<String>,
    format: (Long) -> String,
) {
    Column(
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        modifier = Modifier.fillMaxSize().padding(horizontal = TetherSpace.space4),
    ) {
        if (preview.truncated) {
            val size = truncatedSizeText(preview, format)
            Text(
                text = stringResource(R.string.files_view_truncated, size),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
        CodeLines(lines = lines, modifier = Modifier.fillMaxWidth().weight(1f))
    }
}

/**
 * The lines in one horizontally scrollable box. The list gets a fixed width from the longest
 * line, measured once in the code font, so the scroll range does not jump while the list scrolls.
 */
@Composable
private fun CodeLines(
    lines: List<String>,
    modifier: Modifier,
) {
    val measurer = rememberTextMeasurer()
    val density = LocalDensity.current
    val codeStyle: TextStyle = TetherTheme.mono
    val longest = remember(lines) { lines.maxByOrNull { it.length }.orEmpty() }
    val codeWidth: Dp =
        remember(longest, codeStyle, density) {
            with(density) {
                measurer
                    .measure(AnnotatedString(longest), codeStyle, softWrap = false)
                    .size.width
                    .toDp()
            }
        }
    // Each row is: pad, gutter, gap, code, pad.
    val listWidth = codeWidth + gutterWidth + TetherSpace.space6
    Box(
        modifier =
            modifier
                .background(TetherTheme.colors.codeBg, RoundedCornerShape(TetherRadius.sm))
                .padding(vertical = TetherSpace.space2)
                .horizontalScroll(rememberScrollState()),
    ) {
        LazyColumn(modifier = Modifier.width(listWidth).fillMaxSize()) {
            itemsIndexed(lines) { index, line ->
                CodeLine(number = index + 1, text = line, style = codeStyle)
            }
        }
    }
}

@Composable
private fun CodeLine(
    number: Int,
    text: String,
    style: TextStyle,
) {
    Row(modifier = Modifier.padding(horizontal = TetherSpace.space2)) {
        Text(
            text = number.toString(),
            style = style,
            color = TetherTheme.colors.text3,
            textAlign = TextAlign.End,
            maxLines = 1,
            modifier = Modifier.width(gutterWidth),
        )
        Spacer(Modifier.width(TetherSpace.space2))
        Text(
            text = text,
            style = style,
            color = TetherTheme.colors.text,
            softWrap = false,
            maxLines = 1,
        )
    }
}
