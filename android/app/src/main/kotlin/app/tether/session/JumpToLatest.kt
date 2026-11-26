package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.res.pluralStringResource
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
 * The pill that jumps to the newest row. It shows when [count] is above zero, which means the
 * reader scrolled up and rows arrived since. A tap calls [onClick].
 */
@Composable
fun JumpToLatest(
    count: Int,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    if (count <= 0) return
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
