package app.tether.nav

import org.junit.Assert.assertEquals
import org.junit.Test

class DiscardEmptySessionTest {
    private val events = mutableListOf<String>()

    private fun run(
        empty: Boolean,
        deleteFails: Boolean = false,
        checkFails: Boolean = false,
    ) = closeDiscardingEmpty(
        isEmpty = {
            check(!checkFails) { "check failed" }
            empty
        },
        delete = {
            events += "delete"
            check(!deleteFails) { "delete failed" }
        },
        close = { events += "close" },
        failed = { events += "failed" },
    )

    @Test
    fun an_empty_session_is_deleted_and_then_closed() {
        run(empty = true)

        assertEquals(listOf("delete", "close"), events)
    }

    @Test
    fun a_session_with_messages_is_only_closed() {
        run(empty = false)

        assertEquals(listOf("close"), events)
    }

    @Test
    fun a_failed_delete_is_reported_and_the_session_still_closes() {
        run(empty = true, deleteFails = true)

        assertEquals(listOf("delete", "failed", "close"), events)
    }

    @Test
    fun a_failed_check_is_reported_and_the_session_still_closes() {
        run(empty = true, checkFails = true)

        assertEquals(listOf("failed", "close"), events)
    }
}
