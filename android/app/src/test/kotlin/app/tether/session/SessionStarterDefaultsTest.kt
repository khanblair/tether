package app.tether.session

import app.tether.core.generated.PermissionModeView
import app.tether.core.generated.ProviderView
import app.tether.providers.FakeProviderGateway
import app.tether.settings.DefaultModel
import app.tether.settings.ProjectDefaults
import app.tether.settings.ProjectDefaultsGateway
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SessionStarterDefaultsTest {
    private val providers = FakeProviderGateway()
    private val sessions = FakeSessionGateway()
    private val projectDefaults = MapProjectDefaults()
    private val appDefault = FixedAppDefault()
    private val starter = SessionStarter(providers, sessions, projectDefaults, appDefault)

    @Test
    fun the_project_default_wins_when_its_provider_has_a_key() {
        providers.keys[ProviderView.OPEN_AI] = "key"
        providers.keys[ProviderView.ANTHROPIC] = "key"
        projectDefaults.saved["p1"] = ProjectDefaults(ProviderView.OPEN_AI, "gpt-project", null)
        appDefault.model = DefaultModel(ProviderView.ANTHROPIC, "claude-app")

        starter.start("p1")

        assertEquals(listOf(Triple("p1", ProviderView.OPEN_AI, "gpt-project")), sessions.created)
    }

    @Test
    fun a_project_default_whose_provider_has_no_key_is_skipped_for_the_app_default() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        projectDefaults.saved["p1"] = ProjectDefaults(ProviderView.OPEN_AI, "gpt-project", null)
        appDefault.model = DefaultModel(ProviderView.ANTHROPIC, "claude-app")

        starter.start("p1")

        assertEquals(listOf(Triple("p1", ProviderView.ANTHROPIC, "claude-app")), sessions.created)
    }

    @Test
    fun the_app_default_is_used_when_the_project_has_none() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "first-saved"
        appDefault.model = DefaultModel(ProviderView.ANTHROPIC, "claude-app")

        starter.start("p1")

        assertEquals(listOf(Triple("p1", ProviderView.ANTHROPIC, "claude-app")), sessions.created)
    }

    @Test
    fun an_app_default_whose_provider_has_no_key_falls_through_to_the_first_keyed_provider() {
        providers.keys[ProviderView.GEMINI] = "key"
        sessions.models[ProviderView.GEMINI] = "gemini-first"
        appDefault.model = DefaultModel(ProviderView.OPEN_AI, "gpt-app")

        starter.start("p1")

        assertEquals(listOf(Triple("p1", ProviderView.GEMINI, "gemini-first")), sessions.created)
    }

    @Test
    fun a_blank_default_model_id_is_skipped() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "first-saved"
        projectDefaults.saved["p1"] = ProjectDefaults(ProviderView.ANTHROPIC, "   ", null)
        appDefault.model = DefaultModel(ProviderView.ANTHROPIC, "  ")

        starter.start("p1")

        assertEquals(listOf(Triple("p1", ProviderView.ANTHROPIC, "first-saved")), sessions.created)
    }

    @Test
    fun a_project_default_with_no_model_id_is_skipped() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "first-saved"
        projectDefaults.saved["p1"] = ProjectDefaults(ProviderView.OPEN_AI, null, null)

        starter.start("p1")

        assertEquals(listOf(Triple("p1", ProviderView.ANTHROPIC, "first-saved")), sessions.created)
    }

    @Test
    fun a_default_is_trimmed_before_the_session_is_created() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        appDefault.model = DefaultModel(ProviderView.ANTHROPIC, "  claude-app  ")

        starter.start("p1")

        assertEquals("claude-app", sessions.created.single().third)
    }

    @Test
    fun no_default_and_no_key_still_gives_no_key() {
        val result = starter.start("p1")

        assertEquals(StartResult.NoKey, result)
        assertTrue(sessions.created.isEmpty())
    }

    @Test
    fun the_project_mode_wins_over_the_app_mode() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        projectDefaults.saved["p1"] =
            ProjectDefaults(ProviderView.ANTHROPIC, "claude", SessionMode.PLAN)
        appDefault.mode = SessionMode.AUTO

        starter.start("p1")

        assertEquals(listOf("new-session" to PermissionModeView.PLAN), sessions.modes)
    }

    @Test
    fun the_app_mode_applies_when_the_project_has_none() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "first-saved"
        appDefault.mode = SessionMode.ASK

        starter.start("p1")

        assertEquals(listOf("new-session" to PermissionModeView.ASK), sessions.modes)
    }

    @Test
    fun no_saved_mode_makes_no_mode_call() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "first-saved"

        starter.start("p1")

        assertTrue(sessions.modes.isEmpty())
    }

    @Test
    fun a_refused_mode_call_still_starts_the_session_in_the_core_mode() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "first-saved"
        appDefault.mode = SessionMode.AUTO
        sessions.setModeError = IllegalStateException("core refused the mode")

        val result = starter.start("p1")

        assertEquals(StartResult.Started("new-session"), result)
        assertEquals(1, sessions.created.size)
        assertTrue(sessions.modes.isEmpty())
    }

    @Test
    fun a_bypass_app_default_skips_the_mode_call() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "first-saved"
        appDefault.mode = SessionMode.BYPASS

        val result = starter.start("p1")

        assertEquals(StartResult.Started("new-session"), result)
        assertEquals(1, sessions.created.size)
        assertTrue(sessions.modes.isEmpty())
    }

    @Test
    fun a_bypass_project_default_skips_the_mode_call_even_with_an_app_mode() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        projectDefaults.saved["p1"] =
            ProjectDefaults(ProviderView.ANTHROPIC, "claude", SessionMode.BYPASS)
        appDefault.mode = SessionMode.AUTO

        starter.start("p1")

        assertTrue(sessions.modes.isEmpty())
    }

    @Test
    fun the_normal_modes_are_applied_as_saved() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "first-saved"
        listOf(SessionMode.PLAN, SessionMode.ACCEPT_EDITS, SessionMode.AUTO).forEach { mode ->
            appDefault.mode = mode
            starter.start("p1")
        }

        assertEquals(
            listOf(
                "new-session" to PermissionModeView.PLAN,
                "new-session" to PermissionModeView.ACCEPT_EDITS,
                "new-session" to PermissionModeView.AUTO,
            ),
            sessions.modes,
        )
    }

    @Test
    fun a_project_default_that_cannot_be_read_gives_failed_and_creates_nothing() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        projectDefaults.failLoad = true

        val result = starter.start("p1")

        assertEquals(StartResult.Failed, result)
        assertTrue(sessions.created.isEmpty())
    }

    private class FixedAppDefault : app.tether.settings.AppDefaultSource {
        var model: DefaultModel? = null
        var mode: SessionMode? = null

        override fun defaultModel(): DefaultModel? = model

        override fun savedDefaultMode(): SessionMode? = mode
    }

    private class MapProjectDefaults : ProjectDefaultsGateway {
        val saved = mutableMapOf<String, ProjectDefaults>()
        var failLoad = false

        override fun load(projectId: String): ProjectDefaults {
            check(!failLoad) { "load failed" }
            return saved[projectId] ?: ProjectDefaults(null, null, null)
        }

        override fun save(
            projectId: String,
            defaults: ProjectDefaults,
        ) {
            saved[projectId] = defaults
        }
    }
}
