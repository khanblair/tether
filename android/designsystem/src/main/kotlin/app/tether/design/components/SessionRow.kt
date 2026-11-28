package app.tether.design.components

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

/**
 * A session in the drawer: a [StatusMark], the title on one line that ends in an ellipsis, and a
 * subtitle such as "Needs approval". The row is one button for TalkBack, and the mark is
 * decorative. [onLongClick] is optional.
 */
@Composable
fun SessionRow(
    title: String,
    subtitle: String,
    state: SessionState,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    onLongClick: (() -> Unit)? = null,
) {
    ListRow(
        title = title,
        subtitle = subtitle,
        modifier = modifier,
        leading = { StatusMark(state = state) },
        onClick = onClick,
        onLongClick = onLongClick,
        titleMaxLines = 1,
    )
}
