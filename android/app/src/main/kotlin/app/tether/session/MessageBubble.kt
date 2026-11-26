package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace

/** The widest a user bubble grows, as a share of the row. */
private const val BUBBLE_MAX_RATIO = 0.85f

/**
 * A user message: a neutral bubble on the right. Its text is plain, with newlines kept. A long
 * press opens the message actions when [onLongPress] is set.
 */
@Composable
internal fun UserBubble(
    message: MessageView,
    modifier: Modifier = Modifier,
    onLongPress: (() -> Unit)? = null,
    onCopy: (() -> Unit)? = null,
    onEdit: (() -> Unit)? = null,
) {
    val pressModifier =
        if (onLongPress == null) {
            Modifier
        } else {
            Modifier.combinedClickable(
                onClick = {},
                onLongClickLabel = stringResource(R.string.states_message_actions),
                onLongClick = onLongPress,
            )
        }
    BoxWithConstraints(modifier = modifier.padding(horizontal = TetherSpace.space4)) {
        Column(
            horizontalAlignment = Alignment.End,
            modifier = Modifier.align(Alignment.TopEnd).widthIn(max = maxWidth * BUBBLE_MAX_RATIO),
        ) {
            Column(
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
                modifier =
                    Modifier
                        .clip(RoundedCornerShape(TetherRadius.lg))
                        .background(TetherTheme.colors.surfaceSunken)
                        .then(pressModifier)
                        .padding(TetherSpace.space3),
            ) {
                message.parts.forEach { part -> UserPart(part) }
            }
            MessageActionRow(
                copyLabel = stringResource(R.string.chat_copy_message),
                onCopy = onCopy?.takeIf { plainText(message).isNotBlank() },
                onEdit = onEdit,
                alignEnd = true,
            )
        }
    }
}

/** One part of a user message: text as typed, anything else as the attachment line. */
@Composable
private fun UserPart(part: MessagePartView) {
    when (part) {
        is MessagePartView.Text ->
            Text(
                text = part.text,
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text,
            )
        else -> MessagePartBody(part)
    }
}
