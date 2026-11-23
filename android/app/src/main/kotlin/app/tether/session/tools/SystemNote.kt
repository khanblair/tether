package app.tether.session.tools

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace

/**
 * A quiet line in the middle of the transcript, between two rules: a compaction, a rewind, or a
 * mode or model change. Numbers use thousands separators and units.
 */
@Composable
fun SystemNote(
    kind: SystemNoteKind,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    val locale = LocalConfiguration.current.locales[0]
    val text =
        when (kind) {
            is SystemNoteKind.Compacted ->
                stringResource(
                    R.string.tools_note_compacted,
                    formatTokenCount(kind.tokensBefore, locale),
                    formatTokenCount(kind.tokensAfter, locale),
                )
            SystemNoteKind.Rewound -> stringResource(R.string.tools_note_rewound)
            is SystemNoteKind.ModeChanged -> stringResource(R.string.tools_note_mode, kind.mode)
            is SystemNoteKind.ModelChanged -> stringResource(R.string.tools_note_model, kind.model)
        }
    Row(
        modifier = modifier.fillMaxWidth().padding(vertical = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        HorizontalDivider(
            modifier = Modifier.weight(1f),
            thickness = TetherBorder.hairline,
            color = colors.line,
        )
        Text(
            text = text,
            style = MaterialTheme.typography.labelSmall,
            color = colors.text3,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(horizontal = TetherSpace.space2),
        )
        HorizontalDivider(
            modifier = Modifier.weight(1f),
            thickness = TetherBorder.hairline,
            color = colors.line,
        )
    }
}
