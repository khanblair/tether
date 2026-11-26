package app.tether.di

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class RunQuietlyTest {
    @Test
    fun a_failing_block_is_reported_and_does_not_escape() {
        val reported = mutableListOf<Exception>()

        runQuietly(block = { throw IllegalStateException("closed twice") }) { reported += it }

        assertEquals(listOf("IllegalStateException"), reported.map { it.javaClass.simpleName })
    }

    @Test
    fun a_successful_block_reports_nothing() {
        var ran = false
        val reported = mutableListOf<Exception>()

        runQuietly(block = { ran = true }) { reported += it }

        assertTrue(ran)
        assertTrue(reported.isEmpty())
    }
}
