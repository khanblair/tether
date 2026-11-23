package app.tether.git

import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * One chip per file in the diff, with its counts. The row scrolls sideways. A tap calls
 * [onPick] with the file's index, and the screen scrolls the diff to that file.
 */
@Composable
fun DiffFileChips(
    files: List<DiffFile>,
    onPick: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    LazyRow(
        modifier = modifier,
        contentPadding = PaddingValues(horizontal = TetherSpace.space4),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        itemsIndexed(files, key = { index, file -> "$index:${file.path}" }) { index, file ->
            FileChip(file = file, onClick = { onPick(index) })
        }
    }
}

/** One file chip: its path in mono, then the counts. The chip is at least 48 dp tall. */
@Composable
private fun FileChip(
    file: DiffFile,
    onClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .heightIn(min = TetherSize.touchMin)
                .border(
                    TetherBorder.input,
                    TetherTheme.colors.lineInput,
                    RoundedCornerShape(TetherRadius.sm),
                ).clickable(role = Role.Button, onClick = onClick)
                .padding(horizontal = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        Text(
            text = file.path,
            style = TetherTheme.monoSmall,
            color = TetherTheme.colors.text,
            maxLines = 1,
            overflow = TextOverflow.MiddleEllipsis,
        )
        FileCounts(added = file.added, removed = file.removed)
    }
}
