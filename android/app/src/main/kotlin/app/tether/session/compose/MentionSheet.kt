package app.tether.session.compose

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * The `@` file picker above the composer. It lists project files that match the typed word, one
 * per row, with the path in mono and shortened in the middle. Picking a file calls [onPick].
 */
@Composable
fun MentionSheet(
    files: List<String>,
    onPick: (String) -> Unit,
    modifier: Modifier = Modifier,
    loading: Boolean = false,
    failed: Boolean = false,
) {
    AssistPanel(title = stringResource(R.string.compose_mention_title), modifier = modifier) {
        when {
            failed -> AssistMessage(stringResource(R.string.compose_mention_failed))
            loading && files.isEmpty() ->
                AssistMessage(
                    stringResource(R.string.compose_mention_loading),
                )
            files.isEmpty() -> AssistMessage(stringResource(R.string.compose_mention_empty))
            else ->
                LazyColumn(modifier = Modifier.heightIn(max = TetherSize.notesMax)) {
                    items(files, key = { it }) { path ->
                        MentionRow(path = path, onClick = { onPick(path) })
                    }
                }
        }
    }
}

/** One file row: a file icon, then the path in mono, cut in the middle when it is long. */
@Composable
private fun MentionRow(
    path: String,
    onClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(role = Role.Button, onClick = onClick)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TetherIcon(
            icon = TetherIcons.FileText,
            contentDescription = null,
            tint = TetherTheme.colors.text2,
        )
        Spacer(Modifier.width(TetherSpace.space3))
        Text(
            text = truncateMiddle(path),
            style = TetherTheme.monoLabel,
            color = TetherTheme.colors.text,
            maxLines = 1,
            overflow = TextOverflow.Clip,
        )
    }
}
