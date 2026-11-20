package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import app.tether.R
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import app.tether.design.TetherTheme
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** Persisted messages are read off the main thread when the session route opens. */
@Composable
fun TranscriptScreen(
    connection: SessionConnection?,
    onBack: () -> Unit,
) {
    var messages by remember(connection) { mutableStateOf<List<MessageView>>(emptyList()) }
    var failed by remember(connection) { mutableStateOf(false) }
    var running by remember(connection) { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(connection) {
        if (connection != null) {
            try {
                messages = withContext(Dispatchers.IO) { connection.transcript() }
                    .filterNot(MessageView::rewound)
                    .sortedBy(MessageView::seq)
                connection.changes.collect { event ->
                    when (event) {
                        is CoreEventView.TurnFinished, is CoreEventView.TurnFailed -> running = false
                        else -> Unit
                    }
                    messages = withContext(Dispatchers.IO) { connection.transcript() }
                        .filterNot(MessageView::rewound)
                        .sortedBy(MessageView::seq)
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                failed = true
            }
        }
    }
    TranscriptContent(
        messages = messages,
        failed = failed,
        onBack = onBack,
        composer = if (connection == null) null else {
            {
                Composer(
                    modelId = connection.modelId,
                    running = running,
                    onSend = { text ->
                        running = true
                        scope.launch(Dispatchers.IO) {
                            try {
                                connection.send(text)
                            } catch (_: Exception) {
                                running = false
                                failed = true
                            }
                        }
                    },
                    onStop = { scope.launch(Dispatchers.IO) { connection.stop() } },
                )
            }
        },
    )
}

/** Lazy items use persisted IDs so rows keep their identity as text arrives. */
@Composable
fun TranscriptContent(
    messages: List<MessageView>,
    failed: Boolean,
    onBack: () -> Unit,
    composer: (@Composable () -> Unit)? = null,
) {
    val listState = rememberLazyListState()
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.route_session_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
        Box(modifier = Modifier.weight(1f)) {
            if (messages.isEmpty()) {
                Text(
                    text =
                        stringResource(
                            if (failed) R.string.transcript_load_failed else R.string.transcript_empty,
                        ),
                    color = TetherTheme.colors.text2,
                    modifier = Modifier.padding(TetherSpace.space4),
                )
            } else {
                LazyColumn(
                    state = listState,
                    verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    modifier = Modifier.fillMaxSize(),
                ) {
                    items(messages, key = MessageView::id) { message ->
                        TranscriptMessage(
                            message,
                            Modifier.widthIn(max = TetherSize.transcriptMax).fillMaxWidth(),
                        )
                    }
                }
            }
        }
        composer?.invoke()
        }
    }
}

@Composable
private fun TranscriptMessage(
    message: MessageView,
    modifier: Modifier = Modifier,
) {
    val label =
        when (message.role) {
            RoleView.USER -> R.string.transcript_you
            RoleView.ASSISTANT -> R.string.transcript_agent
            RoleView.TOOL -> R.string.transcript_tool
            RoleView.SYSTEM_NOTE -> R.string.transcript_system
        }
    Column(
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        modifier = modifier.padding(horizontal = TetherSpace.space4),
    ) {
        Text(
            stringResource(label),
            style = MaterialTheme.typography.labelMedium,
            color = TetherTheme.colors.text2,
        )
        message.parts.forEach { part ->
            when (part) {
                is MessagePartView.Text -> MarkdownBlocks(part.text)
                is MessagePartView.Thinking -> Text(part.text, color = TetherTheme.colors.text2)
                is MessagePartView.CompactionSummary ->
                    Text(part.text, color = TetherTheme.colors.text2)
                else ->
                    Text(
                        stringResource(R.string.transcript_attachment),
                        color = TetherTheme.colors.text2,
                    )
            }
        }
    }
}

@Composable
private fun MarkdownBlocks(source: String) {
    splitMarkdown(source).forEach { block ->
        if (block.code) {
            Text(
                text = block.text,
                style = TetherTheme.mono,
                color = TetherTheme.colors.text,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(TetherTheme.colors.surfaceSunken)
                        .padding(TetherSpace.space3),
            )
        } else {
            Text(
                text = inlineMarkdown(block.text),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text,
            )
        }
    }
}

internal data class MarkdownBlock(
    val text: String,
    val code: Boolean,
)

/** Keeps fenced code separate from prose; incomplete fences stay visible while streaming. */
internal fun splitMarkdown(source: String): List<MarkdownBlock> {
    val result = mutableListOf<MarkdownBlock>()
    val text = StringBuilder()
    var inCode = false
    source.lineSequence().forEach { line ->
        if (line.startsWith("```")) {
            if (text.isNotEmpty()) result += MarkdownBlock(text.toString().trimEnd('\n'), inCode)
            text.clear()
            inCode = !inCode
        } else {
            text.append(line).append('\n')
        }
    }
    if (text.isNotEmpty()) result += MarkdownBlock(text.toString().trimEnd('\n'), inCode)
    return result
}

/** Styles bold spans while leaving malformed delimiters as plain text. */
internal fun inlineMarkdown(source: String): AnnotatedString =
    buildAnnotatedString {
        var cursor = 0
        while (cursor < source.length) {
            val start = source.indexOf("**", cursor)
            val end = if (start >= 0) source.indexOf("**", start + 2) else -1
            if (end < 0) {
                append(source.substring(cursor))
                break
            }
            append(source.substring(cursor, start))
            pushStyle(SpanStyle(fontWeight = FontWeight.Bold))
            append(source.substring(start + 2, end))
            pop()
            cursor = end + 2
        }
    }
