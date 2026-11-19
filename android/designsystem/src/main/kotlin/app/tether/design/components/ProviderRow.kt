package app.tether.design.components

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme

/**
 * One provider in a settings list: a status icon, the name and a subtitle, and a chevron.
 * [statusDescription] is read by TalkBack as the row's state, such as "Key rejected".
 */
@Composable
fun ProviderRow(
    name: String,
    subtitle: String,
    status: ProviderRowStatus,
    onClick: () -> Unit,
    statusDescription: String,
    modifier: Modifier = Modifier,
) {
    ListRow(
        title = name,
        subtitle = subtitle,
        modifier = modifier.semantics { stateDescription = statusDescription },
        leading = { StatusIcon(status) },
        trailing = {
            TetherIcon(
                icon = TetherIcons.ChevronRight,
                contentDescription = null,
                tint = TetherTheme.colors.text3,
            )
        },
        onClick = onClick,
    )
}

@Composable
private fun StatusIcon(status: ProviderRowStatus) {
    val (icon, tint) =
        when (status) {
            ProviderRowStatus.Unset -> TetherIcons.CircleDot to TetherTheme.colors.text3
            ProviderRowStatus.Ok -> TetherIcons.CircleCheck to TetherTheme.colors.success
            ProviderRowStatus.BadKey, ProviderRowStatus.Blocked ->
                TetherIcons.CircleX to TetherTheme.colors.danger
            ProviderRowStatus.NetworkError -> TetherIcons.CircleX to TetherTheme.colors.warning
        }
    TetherIcon(icon = icon, contentDescription = null, tint = tint)
}
