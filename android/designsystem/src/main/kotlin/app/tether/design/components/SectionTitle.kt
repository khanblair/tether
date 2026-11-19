package app.tether.design.components

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSpace

/** A section heading in a list, marked as a heading for TalkBack navigation. */
@Composable
fun SectionTitle(
    text: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelMedium,
        color = TetherTheme.colors.text3,
        modifier =
            modifier
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space2)
                .semantics { heading() },
    )
}
