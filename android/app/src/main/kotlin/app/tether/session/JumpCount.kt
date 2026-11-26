package app.tether.session

/**
 * The list rows added since the reader left the bottom, for the jump-to-latest pill. It is zero
 * while the reader is at the bottom, and zero when the list did not grow.
 */
internal fun newItemCount(
    itemsWhenLeft: Int,
    itemsNow: Int,
    atBottom: Boolean,
): Int = if (atBottom) 0 else (itemsNow - itemsWhenLeft).coerceAtLeast(0)
