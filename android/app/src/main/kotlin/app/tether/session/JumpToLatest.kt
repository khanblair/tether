package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.minimumInteractiveComponentSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import app.tether.R
import app.tether.design.ArrowDown
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherElevation
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * The control that jumps to the newest row. [pill] picks what shows: nothing at the bottom, a
 * round icon when the reader is above the bottom and no rows arrived, or the count pill when rows
 * arrived since. A tap calls [onClick].
 */
@Composable
internal fun JumpToLatest(
    pill: JumpPill,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    when (pill) {
        JumpPill.Hidden -> Unit
        JumpPill.Icon -> JumpIconButton(onClick = onClick, modifier = modifier)
        is JumpPill.Count -> JumpCountPill(pill.count, onClick, modifier)
    }
}

/** The round button with no count. The touch target is at least 48 dp; the circle is 40 dp. */
@Composable
private fun JumpIconButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val description = stringResource(R.string.jump_to_latest)
    Box(
        modifier =
            modifier
                .minimumInteractiveComponentSize()
                .clickable(role = Role.Button, onClickLabel = description, onClick = onClick)
                .semantics { contentDescription = description },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier =
                Modifier
                    .size(TetherSpace.space10)
                    .shadow(TetherElevation.level2, CircleShape)
                    .clip(CircleShape)
                    .background(TetherTheme.colors.surfaceRaised),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                imageVector = TetherIcons.ArrowDown,
                contentDescription = null,
                modifier = Modifier.size(TetherSize.iconS),
                tint = TetherTheme.colors.text2,
            )
        }
    }
}

/** The pill with the count of rows that arrived since the reader left the bottom. */
@Composable
private fun JumpCountPill(
    count: Int,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val label = pluralStringResource(R.plurals.session_jump_new, count, count)
    val description = pluralStringResource(R.plurals.session_jump_description, count, count)
    Row(
        modifier =
            modifier
                .shadow(TetherElevation.level2, RoundedCornerShape(TetherRadius.full))
                .clip(RoundedCornerShape(TetherRadius.full))
                .background(TetherTheme.colors.surfaceRaised)
                .clickable(role = Role.Button, onClickLabel = description, onClick = onClick)
                .semantics { contentDescription = description }
                .heightIn(min = TetherSpace.space10)
                .padding(horizontal = TetherSpace.space4),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = TetherIcons.ArrowDown,
            contentDescription = null,
            modifier = Modifier.size(TetherSize.iconS),
            tint = TetherTheme.colors.text2,
        )
        Spacer(Modifier.width(TetherSpace.space2))
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = TetherTheme.colors.text,
        )
    }
}

/** Scrolls to the newest row. Reduced motion jumps instead of animating. */
internal suspend fun scrollToLatest(
    listState: LazyListState,
    reducedMotion: Boolean,
) {
    val last = listState.layoutInfo.totalItemsCount - 1
    if (last < 0) return
    if (reducedMotion) {
        listState.scrollToItem(last, Int.MAX_VALUE)
    } else {
        listState.animateScrollToItem(last, Int.MAX_VALUE)
    }
}
