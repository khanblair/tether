package app.tether.session

import app.tether.core.generated.ProviderView
import app.tether.providers.FakeProviderGateway
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SessionStarterTest {
    private val providers = FakeProviderGateway()
    private val sessions = FakeSessionGateway()
    private val starter = SessionStarter(providers, sessions)

    @Test
    fun the_first_provider_in_order_with_a_key_starts_the_session() {
        providers.keys[ProviderView.GEMINI] = "key"
        providers.keys[ProviderView.OPEN_AI] = "key"
        sessions.models[ProviderView.OPEN_AI] = "gpt-model"
        sessions.models[ProviderView.GEMINI] = "gemini-model"

        val result = starter.start("project-1")

        assertEquals(StartResult.Started("new-session"), result)
        assertEquals(
            listOf(Triple("project-1", ProviderView.OPEN_AI, "gpt-model")),
            sessions.created,
        )
    }

    @Test
    fun the_model_name_is_trimmed_before_the_session_is_created() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "  claude-model  "

        starter.start("project-1")

        assertEquals("claude-model", sessions.created.single().third)
    }

    @Test
    fun no_key_gives_no_key_and_creates_nothing() {
        val result = starter.start("project-1")

        assertEquals(StartResult.NoKey, result)
        assertTrue(sessions.created.isEmpty())
    }

    @Test
    fun a_missing_model_list_gives_no_models_and_creates_nothing() {
        providers.keys[ProviderView.ANTHROPIC] = "key"

        val result = starter.start("project-1")

        assertEquals(StartResult.NoModels, result)
        assertTrue(sessions.created.isEmpty())
    }

    @Test
    fun a_blank_model_name_gives_no_models_and_creates_nothing() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "   "

        val result = starter.start("project-1")

        assertEquals(StartResult.NoModels, result)
        assertTrue(sessions.created.isEmpty())
    }

    @Test
    fun a_gateway_exception_gives_failed() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "model"
        sessions.failNext = IllegalStateException("core unavailable")

        val result = starter.start("project-1")

        assertEquals(StartResult.Failed, result)
        assertTrue(sessions.created.isEmpty())
    }
}
