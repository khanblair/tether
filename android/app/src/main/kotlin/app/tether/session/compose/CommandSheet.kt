package app.tether.session.compose

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * The `/` command list above the composer. It lists the commands whose name starts with [query].
 * A command that is not available is dimmed with its reason and cannot be picked. Picking an
 * available one calls [onPick]; the caller replaces the text and does not send.
 */
@Composable
fun CommandSheet(
    commands: List<ComposerCommand>,
    query: String,
    onPick: (ComposerCommand) -> Unit,
    modifier: Modifier = Modifier,
    loaded: Boolean = true,
    failed: Boolean = false,
) {
    val matches = commandsMatching(commands, query)
    AssistPanel(title = stringResource(R.string.compose_commands_title), modifier = modifier) {
        when {
            failed -> AssistMessage(stringResource(R.string.compose_commands_failed))
            !loaded -> AssistMessage(stringResource(R.string.compose_commands_loading))
            matches.isEmpty() -> AssistMessage(stringResource(R.string.compose_command_empty))
            else ->
                LazyColumn(modifier = Modifier.heightIn(max = TetherSize.notesMax)) {
                    items(matches, key = { it.name }) { command ->
                        CommandRow(command = command, onClick = { onPick(command) })
                    }
                }
        }
    }
}

/** One command: the name in mono, its description, and the reason when it is not available. */
@Composable
private fun CommandRow(
    command: ComposerCommand,
    onClick: () -> Unit,
) {
    val reason =
        if (command.available) {
            null
        } else {
            command.requires?.let { stringResource(R.string.compose_command_needs, it) }
                ?: stringResource(R.string.compose_command_unavailable_default)
        }
    val colors = TetherTheme.colors
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(enabled = command.available, role = Role.Button, onClick = onClick)
                .semantics { reason?.let { stateDescription = it } }
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space2),
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            text = "/${command.name}",
            style = TetherTheme.monoLabel,
            color = if (command.available) colors.text else colors.textDisabled,
        )
        Text(
            text = command.description,
            style = MaterialTheme.typography.bodySmall,
            color = if (command.available) colors.text2 else colors.text3,
        )
        reason?.let {
            Text(text = it, style = MaterialTheme.typography.bodySmall, color = colors.text3)
        }
    }
}

/** The shared frame for the composer's sheets: a soft border, a small title, and the content. */
@Composable
internal fun AssistPanel(
    title: String,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .background(TetherTheme.colors.surfaceRaised, shape)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape)
                .padding(vertical = TetherSpace.space2),
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
            modifier =
                Modifier.padding(
                    horizontal = TetherSpace.space4,
                    vertical = TetherSpace.space1,
                ),
        )
        content()
    }
}

/** A one-line state inside a sheet, such as an empty result. */
@Composable
internal fun AssistMessage(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyMedium,
        color = TetherTheme.colors.text2,
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
    )
}
