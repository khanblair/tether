package app.tether.session

import app.tether.core.generated.SessionRuntimeException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class SessionConnectionFallbackTest {
    @Test
    fun theWorkspaceOpensWhenItWorks() {
        var plainCalls = 0

        val result =
            openWithFallback(workspace = { "workspace" }, plain = {
                plainCalls++
                "plain"
            })

        assertEquals("workspace", result)
        assertEquals(0, plainCalls)
    }

    @Test
    fun anUnavailableWorkspaceFallsBackToPlainChat() {
        val result =
            openWithFallback(
                workspace = { throw SessionRuntimeException.Unavailable() },
                plain = { "plain" },
            )

        assertEquals("plain", result)
    }

    @Test
    fun otherErrorsReachTheCallerUnchanged() {
        val error = SessionRuntimeException.NotFound()
        var plainCalls = 0

        val thrown =
            assertThrows(SessionRuntimeException.NotFound::class.java) {
                openWithFallback<String>(
                    workspace = { throw error },
                    plain = {
                        plainCalls++
                        "plain"
                    },
                )
            }

        assertEquals(error, thrown)
        assertEquals(0, plainCalls)
    }
}
