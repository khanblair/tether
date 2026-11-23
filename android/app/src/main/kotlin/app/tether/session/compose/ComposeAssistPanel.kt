package app.tether.session.compose

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import app.tether.design.tokens.TetherSpace

/**
 * The composer's sheets and mention chips, drawn above the text field. [text] and [cursor] are
 * the field's current value. [onEdit] receives each new text the user makes by picking or
 * removing, and the caller writes it into the field.
 */
@Composable
fun ComposeAssistPanel(
    deps: ComposeAssistDeps,
    text: String,
    cursor: Int,
    onEdit: (ComposerEdit) -> Unit,
) {
    val model: ComposeAssistViewModel =
        viewModel(
            key = "compose-assist-${deps.projectId}",
            factory = ComposeAssistViewModel.factory(deps),
        )
    val state by model.state.collectAsState()
    LaunchedEffect(text, cursor) { model.onTextChanged(text, cursor) }
    val mentioned = mentionsIn(text).map { it.path }.distinct()
    Column(
        modifier = Modifier.fillMaxWidth().padding(bottom = TetherSpace.space2),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        if (mentioned.isNotEmpty()) {
            MentionChips(paths = mentioned, onRemove = { onEdit(removeMention(text, it)) })
        }
        when (val mode = state.mode) {
            is AssistMode.Command ->
                CommandSheet(
                    commands = state.commands,
                    query = mode.prefix,
                    loaded = state.commandsLoaded,
                    failed = state.commandsFailed,
                    onPick = { command -> model.pickCommand(command, text, cursor)?.let(onEdit) },
                )
            is AssistMode.Mention ->
                MentionSheet(
                    files = state.mentionPaths,
                    loading = state.mentionLoading,
                    failed = state.mentionFailed,
                    onPick = { path -> model.pickMention(path, text, cursor)?.let(onEdit) },
                )
            AssistMode.None -> Unit
        }
    }
}
