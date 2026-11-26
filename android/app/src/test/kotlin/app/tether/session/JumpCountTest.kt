package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class JumpCountTest {
    @Test
    fun atBottomNothingIsNew() {
        assertEquals(0, newItemCount(itemsWhenLeft = 10, itemsNow = 14, atBottom = true))
    }

    @Test
    fun rowsAddedAfterLeavingTheBottomAreCounted() {
        assertEquals(3, newItemCount(itemsWhenLeft = 10, itemsNow = 13, atBottom = false))
    }

    @Test
    fun noRowsAddedMeansNothingIsNew() {
        assertEquals(0, newItemCount(itemsWhenLeft = 10, itemsNow = 10, atBottom = false))
    }

    @Test
    fun aShrinkingListNeverCountsNegative() {
        assertEquals(0, newItemCount(itemsWhenLeft = 10, itemsNow = 4, atBottom = false))
    }

    @Test
    fun atBottomIsHiddenEvenWhenRowsArrived() {
        assertEquals(
            JumpPill.Hidden,
            jumpPillFor(atBottom = true, itemsWhenLeft = 10, itemsNow = 14),
        )
    }

    @Test
    fun awayWithNoNewRowsIsIcon() {
        assertEquals(
            JumpPill.Icon,
            jumpPillFor(atBottom = false, itemsWhenLeft = 10, itemsNow = 10),
        )
    }

    @Test
    fun awayWithNewRowsIsCount() {
        assertEquals(
            JumpPill.Count(3),
            jumpPillFor(atBottom = false, itemsWhenLeft = 10, itemsNow = 13),
        )
    }

    @Test
    fun awayWithAShrinkingListIsIcon() {
        assertEquals(
            JumpPill.Icon,
            jumpPillFor(atBottom = false, itemsWhenLeft = 10, itemsNow = 4),
        )
    }
}
