package app.tether.session.compose

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIconButton
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace

/**
 * The mentions in the draft, as small chips above the text field. Each chip's remove button
 * calls [onRemove] with its path; the caller deletes that mention from the text.
 */
@Composable
fun MentionChips(
    paths: List<String>,
    onRemove: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    LazyRow(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        items(paths, key = { it }) { path ->
            MentionChip(path = path, onRemove = { onRemove(path) })
        }
    }
}

/** One chip: the path in mono, and a remove button with a 48 dp target. */
@Composable
private fun MentionChip(
    path: String,
    onRemove: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .background(TetherTheme.colors.surface, RoundedCornerShape(TetherRadius.sm))
                .border(
                    TetherBorder.hairline,
                    TetherTheme.colors.line,
                    RoundedCornerShape(TetherRadius.sm),
                ).padding(start = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = truncateMiddle(path),
            style = TetherTheme.monoSmall,
            color = TetherTheme.colors.text,
        )
        TetherIconButton(
            icon = TetherIcons.X,
            contentDescription = stringResource(R.string.compose_mention_remove, path),
            onClick = onRemove,
        )
    }
}
