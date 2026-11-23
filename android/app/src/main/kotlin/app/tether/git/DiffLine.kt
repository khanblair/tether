package app.tether.git

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.style.TextAlign
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherColors
import app.tether.design.tokens.TetherSpace

/**
 * One diff line: two line-number gutters (old, new), a `+` or `−` mark, and the text. Added and
 * removed lines get a soft background from the diff tokens. TalkBack hears the line as one item,
 * with "Added line" or "Removed line" as its state. With [wrap] off, the text scrolls sideways on
 * its own scroll state, so each long line scrolls independently.
 */
@Composable
fun DiffLine(
    line: DiffLineModel,
    wrap: Boolean,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    val scroll = rememberScrollState()
    val palette = linePalette(line.kind, colors)
    val isNote = line.kind == DiffLineKind.Note
    val textStyle =
        if (isNote) {
            TetherTheme.monoSmall.copy(fontStyle = FontStyle.Italic)
        } else {
            TetherTheme.monoSmall
        }
    val textModifier =
        Modifier
            .padding(end = TetherSpace.space2)
            .then(if (wrap) Modifier else Modifier.horizontalScroll(scroll))
    val label = palette.label?.let { stringResource(it) }
    val rowModifier =
        modifier
            .fillMaxWidth()
            .background(palette.background)
            .semantics(mergeDescendants = true) {
                label?.let { stateDescription = it }
            }

    Row(modifier = rowModifier) {
        LineNumber(line.oldNo)
        LineNumber(line.newNo)
        Text(
            text = palette.mark,
            style = TetherTheme.monoSmall,
            color = palette.markColor,
            textAlign = TextAlign.Center,
            modifier = Modifier.width(TetherSpace.space4),
        )
        Text(
            text = if (isNote) stringResource(R.string.git_diff_no_newline) else line.text,
            style = textStyle,
            color = if (isNote) colors.text3 else colors.text,
            softWrap = wrap,
            modifier = Modifier.weight(1f).then(textModifier),
        )
    }
}

/** The colors, mark, and TalkBack label of one kind of diff line. */
private data class LinePalette(
    val background: Color,
    val markColor: Color,
    val mark: String,
    val label: Int?,
)

/** The palette for [kind]. Added and removed lines carry their token colors and a glyph. */
private fun linePalette(
    kind: DiffLineKind,
    colors: TetherColors,
): LinePalette =
    when (kind) {
        DiffLineKind.Added ->
            LinePalette(
                colors.diffAddBg,
                colors.diffAddMark,
                DIFF_ADDED_MARK,
                R.string.git_diff_line_added,
            )
        DiffLineKind.Removed ->
            LinePalette(
                colors.diffDelBg,
                colors.diffDelMark,
                DIFF_REMOVED_MARK,
                R.string.git_diff_line_removed,
            )
        DiffLineKind.Context, DiffLineKind.Note ->
            LinePalette(Color.Transparent, colors.text3, "", null)
    }

/** One line-number gutter. An empty number, as for an added line's old side, shows nothing. */
@Composable
private fun LineNumber(number: Int?) {
    Text(
        text = number?.toString().orEmpty(),
        style = TetherTheme.monoSmall,
        color = TetherTheme.colors.text3,
        textAlign = TextAlign.End,
        modifier =
            Modifier
                .width(TetherSpace.space10)
                .padding(end = TetherSpace.space1),
    )
}

private const val DIFF_ADDED_MARK = "+"
private const val DIFF_REMOVED_MARK = "−"
