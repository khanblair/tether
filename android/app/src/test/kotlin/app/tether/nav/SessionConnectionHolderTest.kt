package app.tether.nav

import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class SessionConnectionHolderTest {
    private val store = ViewModelStore()
    private val released = mutableListOf<String>()
    private val pending = mutableListOf<suspend () -> Unit>()

    private fun holder(
        sessionId: String?,
        result: SessionOpen<String> = SessionOpen.Opened("connection"),
    ): SessionConnectionHolder<String> =
        ViewModelProvider(
            store,
            viewModelFactory {
                initializer {
                    SessionConnectionHolder(
                        sessionId = sessionId,
                        release = { released += it },
                        launch = { pending += it },
                        open = { result },
                    )
                }
            },
        )[SessionConnectionHolder::class.java] as SessionConnectionHolder<String>

    private fun finishOpening() = runBlocking { pending.forEach { it() } }

    @Test
    fun keeps_the_connection_until_the_entry_leaves_the_back_stack() {
        val holder = holder("session")
        assertSame(SessionOpen.Opening, holder.state.value)

        finishOpening()
        assertEquals("connection", (holder.state.value as SessionOpen.Opened).connection)
        assertTrue("a connection must stay open while the entry lives", released.isEmpty())

        store.clear()
        assertEquals(listOf("connection"), released)
    }

    @Test
    fun closes_a_connection_that_finishes_opening_after_the_entry_is_gone() {
        val holder = holder("session")

        store.clear()
        assertTrue(released.isEmpty())

        finishOpening()
        assertEquals(listOf("connection"), released)
        assertSame(SessionOpen.Opening, holder.state.value)
    }

    @Test
    fun a_missing_session_id_is_unavailable_and_opens_nothing() {
        val holder = holder(null)

        assertNull((holder.state.value as SessionOpen.Unavailable).reason)
        assertTrue(pending.isEmpty())
        store.clear()
        assertTrue(released.isEmpty())
    }

    @Test
    fun an_unavailable_session_releases_nothing() {
        val holder = holder("session", SessionOpen.Unavailable("IllegalStateException"))

        finishOpening()
        val state = holder.state.value as SessionOpen.Unavailable
        assertEquals("IllegalStateException", state.reason)
        store.clear()
        assertTrue(released.isEmpty())
    }
}
