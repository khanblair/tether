package app.tether.session

import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import app.tether.core.generated.MessageView

/**
 * Keeps the newest item in view while the reader is at the bottom. [trailing] counts the items
 * after the messages: the live section and the turn footer. [liveKey] changes with each live
 * update, so a streaming tool card keeps the tail in view. The flag changes only when the scroll
 * position changes, so a streamed reply that grows below the viewport does not count as the
 * reader leaving the bottom. Returns what the jump-to-latest control shows.
 */
@Composable
internal fun followTail(
    listState: LazyListState,
    messages: List<MessageView>,
    trailing: Int,
    liveKey: Any?,
): JumpPill {
    var followTail by remember { mutableStateOf(true) }
    var itemsWhenLeft by remember { mutableIntStateOf(0) }
    var itemsNow by remember { mutableIntStateOf(0) }
    LaunchedEffect(listState) {
        snapshotFlow { listState.firstVisibleItemIndex to listState.firstVisibleItemScrollOffset }
            .collect {
                val atBottom = listState.isAtBottom()
                if (followTail && !atBottom) itemsWhenLeft = listState.layoutInfo.totalItemsCount
                followTail = atBottom
            }
    }
    LaunchedEffect(listState) {
        snapshotFlow { listState.layoutInfo.totalItemsCount }.collect { itemsNow = it }
    }
    // The keyboard shrinks the viewport without moving the scroll position, so a height change
    // also runs the effect below. It scrolls only while the reader is following the tail.
    val viewportHeight by remember(listState) {
        derivedStateOf { listState.layoutInfo.viewportSize.height }
    }
    LaunchedEffect(messages, trailing, liveKey, viewportHeight) {
        val lastItem = messages.size + trailing - 1
        if (lastItem >= 0 && shouldFollow(followTail, hasNewContent = true)) {
            // A large offset clamps to the list end, so a growing reply stays in view.
            listState.scrollToItem(lastItem, Int.MAX_VALUE)
        }
    }
    return jumpPillFor(followTail, itemsWhenLeft, itemsNow)
}

/**
 * True when the last item is fully inside the viewport. There is no slack: the last row must be
 * fully visible, so a reader a few pixels above it sees the button.
 */
internal fun LazyListState.isAtBottom(): Boolean {
    val info = layoutInfo
    val last = info.visibleItemsInfo.lastOrNull() ?: return true
    return last.index == info.totalItemsCount - 1 &&
        last.offset + last.size <= info.viewportEndOffset
}

/** Scroll to the newest content only when the reader was already at the bottom. */
internal fun shouldFollow(
    wasAtBottom: Boolean,
    hasNewContent: Boolean,
): Boolean = wasAtBottom && hasNewContent
