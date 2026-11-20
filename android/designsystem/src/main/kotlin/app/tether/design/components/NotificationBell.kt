package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.clearAndSetSemantics
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace

/**
 * The notification bell in a top bar. When [hasAttention] is true, an accent dot with a bg ring
 * sits at its top right. The dot is decorative, so [contentDescription] carries the count.
 */
@Composable
fun NotificationBell(
    hasAttention: Boolean,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(modifier = modifier) {
        TetherIconButton(
            icon = TetherIcons.Bell,
            contentDescription = contentDescription,
            onClick = onClick,
        )
        if (hasAttention) {
            AttentionDot(modifier = Modifier.align(Alignment.TopEnd))
        }
    }
}

@Composable
private fun AttentionDot(modifier: Modifier) {
    Box(
        modifier =
            modifier
                .padding(top = TetherSpace.space1, end = TetherSpace.space1)
                .background(TetherTheme.colors.bg, CircleShape)
                .padding(TetherBorder.inputFocus)
                .background(TetherTheme.colors.accent, CircleShape)
                .size(TetherSpace.space2)
                .clearAndSetSemantics {},
    )
}
