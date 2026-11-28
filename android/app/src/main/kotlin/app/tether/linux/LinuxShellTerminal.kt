package app.tether.linux

import android.content.ClipData
import android.os.Build
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Density
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSpace
import app.tether.messages.AppMessages
import kotlinx.coroutines.launch

/** Draws the terminal, hidden keyboard field, and the key row above the keyboard. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun ShellTerminal(
    frame: LinuxShellFrame,
    onInput: (ByteArray) -> Unit,
    onResize: (Int, Int) -> Unit,
    onCopy: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()
    val focusRequester = remember { FocusRequester() }
    var input by remember { mutableStateOf("") }
    val latestInput by rememberUpdatedState(onInput)
    val keyboard = remember { TerminalKeyboard { bytes -> latestInput(bytes) } }
    LaunchedEffect(Unit) { focusRequester.requestFocus() }
    LaunchedEffect(Unit) { listState.scrollToItem(frame.lines.lastIndex) }
    val showJump by remember(frame) {
        derivedStateOf { listState.firstVisibleItemIndex < frame.lines.lastIndex }
    }
    BoxWithConstraints(modifier = modifier) {
        val density = LocalDensity.current
        val (rows, columns) = terminalSize(maxWidth, maxHeight, density)
        LaunchedEffect(rows, columns) { onResize(rows, columns) }
        TerminalOutput(frame, listState, onCopy)
        if (showJump) {
            JumpBottomButton(
                onClick = { scope.launch { listState.animateScrollToItem(frame.lines.lastIndex) } },
                modifier = Modifier.align(Alignment.BottomEnd).padding(TetherSpace.space3),
            )
        }
        TerminalInput(input, { input = it }, keyboard, focusRequester)
    }
    KeyRow(keyboard)
}

@Composable
private fun terminalSize(
    maxWidth: androidx.compose.ui.unit.Dp,
    maxHeight: androidx.compose.ui.unit.Dp,
    density: Density,
): Pair<Int, Int> {
    val measurer = rememberTextMeasurer()
    val cellWidth =
        with(density) {
            measurer
                .measure(AnnotatedString("M"), TetherTheme.mono, softWrap = false)
                .size.width
                .toDp()
        }
    val cellHeight =
        with(density) {
            TetherTheme.mono.lineHeight
                .toPx()
                .toDp()
        }
    return (maxHeight / cellHeight).toInt().coerceAtLeast(1) to
        (maxWidth / cellWidth).toInt().coerceAtLeast(1)
}

@Composable
private fun TerminalOutput(
    frame: LinuxShellFrame,
    listState: androidx.compose.foundation.lazy.LazyListState,
    onCopy: (String) -> Unit,
) {
    SelectionContainerWithCopy(text = frame.visibleText(), onCopy = onCopy) {
        LazyColumn(
            state = listState,
            modifier = Modifier.fillMaxSize().background(TetherTheme.colors.codeBg),
        ) {
            itemsIndexed(frame.lines) { index, line ->
                TerminalLineText(
                    line,
                    cursorColumn = frame.cursorColumn,
                    cursor = index == frame.cursorLine,
                )
            }
        }
    }
}

@Composable
private fun TerminalInput(
    value: String,
    onValueChange: (String) -> Unit,
    keyboard: TerminalKeyboard,
    focusRequester: FocusRequester,
) {
    BasicTextField(
        value = value,
        onValueChange = { changed ->
            if (changed.isNotEmpty()) keyboard.type(changed)
            onValueChange("")
        },
        modifier = Modifier.size(TetherSpace.space0_5).alpha(0f).focusRequester(focusRequester),
        keyboardOptions =
            KeyboardOptions(
                keyboardType = KeyboardType.Ascii,
                autoCorrectEnabled = false,
                imeAction = ImeAction.None,
            ),
    )
}

private fun LinuxShellFrame.visibleText(): String =
    lines
        .takeLast(visibleLines)
        .joinToString("\n") { line -> line.joinToString("") { it.character } }

@Composable
private fun TerminalLineText(
    line: TerminalLine,
    cursorColumn: Int,
    cursor: Boolean,
) {
    val builder = AnnotatedString.Builder()
    line.forEachIndexed { index, cell ->
        builder.withStyle(
            cell.style(cursor && index == cursorColumn.coerceAtMost(line.lastIndex)),
        ) {
            append(cell.character)
        }
    }
    Text(
        text = builder.toAnnotatedString(),
        style = TetherTheme.mono,
        maxLines = 1,
        softWrap = false,
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(
                    min =
                        with(LocalDensity.current) {
                            TetherTheme.mono.lineHeight
                                .toPx()
                                .toDp()
                        },
                ),
    )
}

@Composable
private fun KeyRow(keyboard: TerminalKeyboard) {
    val labels = terminalKeyLabels()
    Row(
        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        labels.forEach { (label, key) ->
            TerminalKeyButton(
                label = stringResource(label),
                armed =
                    when (key) {
                        TerminalKey.Ctrl -> keyboard.ctrlArmed
                        TerminalKey.Alt -> keyboard.altArmed
                        else -> false
                    },
                onClick = { keyboard.press(key) },
            )
        }
    }
}

@Composable
private fun TerminalKeyButton(
    label: String,
    armed: Boolean,
    onClick: () -> Unit,
) {
    TextButton(
        onClick = onClick,
        modifier =
            Modifier
                .widthIn(min = TetherSpace.space10)
                .heightIn(min = TetherSpace.space12)
                .then(
                    if (armed) Modifier.background(TetherTheme.colors.accentSoft) else Modifier,
                ).semantics { contentDescription = label },
    ) {
        Text(
            text = label,
            style = TetherTheme.monoLabel,
            color = if (armed) TetherTheme.colors.accent else TetherTheme.colors.text2,
        )
    }
}

private fun terminalKeyLabels(): List<Pair<Int, TerminalKey>> =
    listOf(
        R.string.linux_key_escape to TerminalKey.Escape,
        R.string.linux_key_tab to TerminalKey.Tab,
        R.string.linux_key_ctrl to TerminalKey.Ctrl,
        R.string.linux_key_alt to TerminalKey.Alt,
        R.string.linux_key_up to TerminalKey.ArrowUp,
        R.string.linux_key_down to TerminalKey.ArrowDown,
        R.string.linux_key_left to TerminalKey.ArrowLeft,
        R.string.linux_key_right to TerminalKey.ArrowRight,
        R.string.linux_key_pipe to TerminalKey.Character('|'),
        R.string.linux_key_slash to TerminalKey.Character('/'),
        R.string.linux_key_dash to TerminalKey.Character('-'),
        R.string.linux_key_tilde to TerminalKey.Character('~'),
    )

@Composable
private fun JumpBottomButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    TextButton(
        onClick = onClick,
        modifier = modifier.heightIn(min = TetherSpace.space10),
    ) {
        Text(text = stringResource(R.string.linux_jump_to_latest))
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun SelectionContainerWithCopy(
    text: String,
    onCopy: (String) -> Unit,
    content: @Composable () -> Unit,
) {
    Box(
        modifier =
            Modifier.combinedClickable(
                role = Role.Button,
                onClick = {},
                onLongClick = { onCopy(text) },
            ),
    ) {
        androidx.compose.foundation.text.selection
            .SelectionContainer { content() }
    }
}

@Composable
internal fun rememberCopyTerminal(messages: AppMessages?): (String) -> Unit {
    val clipboard = LocalClipboard.current
    val scope = rememberCoroutineScope()
    val label = stringResource(R.string.linux_copy)
    val copied = stringResource(R.string.linux_copied)
    return { text ->
        scope.launch {
            clipboard.setClipEntry(ClipEntry(ClipData.newPlainText(label, text)))
            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) messages?.show(copied)
        }
    }
}
