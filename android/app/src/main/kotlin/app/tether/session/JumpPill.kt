package app.tether.session

/** What the jump-to-latest control shows. */
internal sealed interface JumpPill {
    /** The reader is at the bottom. Nothing shows. */
    data object Hidden : JumpPill

    /** The reader is above the bottom and no rows arrived since. An icon-only button shows. */
    data object Icon : JumpPill

    /** Rows arrived since the reader left the bottom. The pill shows the count. */
    data class Count(
        val count: Int,
    ) : JumpPill
}

/** Picks the jump control from the scroll state and the row counts. */
internal fun jumpPillFor(
    atBottom: Boolean,
    itemsWhenLeft: Int,
    itemsNow: Int,
): JumpPill {
    if (atBottom) return JumpPill.Hidden
    val new = newItemCount(itemsWhenLeft, itemsNow, atBottom = false)
    return if (new > 0) JumpPill.Count(new) else JumpPill.Icon
}
