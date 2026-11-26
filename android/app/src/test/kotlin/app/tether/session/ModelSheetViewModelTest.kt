package app.tether.session

import app.tether.core.generated.ProviderView
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ModelSheetViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val fixture = SheetFixture()
    private val sonnet =
        ModelChoice("sonnet", "Sonnet 5.5", 200_000, 3_000_000, 15_000_000, true, false, true)
    private val gpt = ModelChoice("gpt-5", "GPT-5", 400_000, null, null, null, null, null)
    private var applied = 0

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): ModelSheetViewModel {
        val vm = ModelSheetViewModel(fixture.deps, onApplied = { applied++ }, io = dispatcher)
        advanceUntilIdle()
        return vm
    }

    @Test
    fun lists_only_providers_with_a_saved_key_and_marks_the_current_model() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            fixture.settings.modelsByProvider[ProviderView.OPEN_AI] = listOf(gpt)
            val state = model().state.value
            assertFalse(state.loading)
            assertEquals(listOf(ProviderView.ANTHROPIC), state.groups.map { it.provider })
            assertEquals(ModelRef(ProviderView.ANTHROPIC, "sonnet"), state.current)
            assertNull(state.failure)
        }

    @Test
    fun no_saved_keys_gives_an_empty_list() =
        runTest(dispatcher) {
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            val state = model().state.value
            assertFalse(state.loading)
            assertTrue(state.groups.isEmpty())
        }

    @Test
    fun a_load_failure_is_kept_as_a_failure() =
        runTest(dispatcher) {
            fixture.settings.failNext = SessionSettingsException(SessionFailure.UNAVAILABLE)
            val state = model().state.value
            assertFalse(state.loading)
            assertEquals(SessionFailure.UNAVAILABLE, state.failure)
        }

    @Test
    fun choosing_a_model_sets_it_reopens_the_runner_and_reports_applied() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            val vm = model()
            vm.choose(ProviderView.ANTHROPIC, sonnet)
            advanceUntilIdle()
            val sets = fixture.settings.calls.filter { it.startsWith("setModel") }
            assertEquals(listOf("setModel:ANTHROPIC:sonnet"), sets)
            assertEquals(1, fixture.reopenCount)
            assertEquals(1, applied)
            assertFalse(vm.state.value.saving)
        }

    @Test
    fun a_turn_in_progress_blocks_the_choice() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            val vm = model()
            fixture.running = true
            vm.choose(ProviderView.ANTHROPIC, sonnet)
            advanceUntilIdle()
            assertTrue(fixture.settings.calls.none { it.startsWith("setModel") })
            assertEquals(0, applied)
        }

    @Test
    fun a_busy_core_keeps_the_sheet_open_with_a_failure() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            val vm = model()
            fixture.settings.failNext = SessionSettingsException(SessionFailure.BUSY)
            vm.choose(ProviderView.ANTHROPIC, sonnet)
            advanceUntilIdle()
            assertEquals(SessionFailure.BUSY, vm.state.value.failure)
            assertEquals(0, fixture.reopenCount)
            assertEquals(0, applied)
        }

    @Test
    fun a_model_without_tool_support_cannot_be_chosen() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.OPEN_AI] = "key"
            val chatOnly =
                ModelChoice("chat-only", "Chat only", null, null, null, false, null, null)
            fixture.settings.modelsByProvider[ProviderView.OPEN_AI] = listOf(chatOnly)
            val vm = model()
            vm.choose(ProviderView.OPEN_AI, chatOnly)
            advanceUntilIdle()
            assertTrue(fixture.settings.calls.none { it.startsWith("setModel") })
            assertEquals(0, fixture.reopenCount)
            assertEquals(0, applied)
        }

    @Test
    fun a_reasoning_model_shows_the_thinking_section_with_the_saved_level() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            fixture.settings.session = fixture.settings.session.copy(effort = ThinkingEffort.LOW)
            val state = model().state.value
            assertTrue(state.showsEffort)
            assertEquals(ThinkingEffort.LOW, state.effort)
        }

    @Test
    fun a_model_that_does_not_reason_hides_the_thinking_section() =
        runTest(dispatcher) {
            val chat = ModelChoice("chat", "Chat", null, null, null, true, null, false)
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet, chat)
            fixture.settings.session = fixture.settings.session.copy(modelId = "chat")
            assertFalse(model().state.value.showsEffort)
        }

    @Test
    fun a_model_with_unknown_reasoning_hides_the_thinking_section() =
        runTest(dispatcher) {
            val unknown = ModelChoice("unknown", "Unknown", null, null, null, true, null, null)
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(unknown)
            fixture.settings.session = fixture.settings.session.copy(modelId = "unknown")
            assertFalse(model().state.value.showsEffort)
        }

    @Test
    fun choosing_a_level_sets_it_reopens_the_runner_and_reports_applied() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            val vm = model()
            vm.chooseEffort(ThinkingEffort.HIGH)
            advanceUntilIdle()
            assertEquals(
                listOf("setEffort:HIGH"),
                fixture.settings.calls.filter { it.startsWith("setEffort") },
            )
            assertEquals(1, fixture.reopenCount)
            assertEquals(1, applied)
            assertFalse(vm.state.value.saving)
        }

    @Test
    fun choosing_the_model_default_sends_none() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            val vm = model()
            vm.chooseEffort(null)
            advanceUntilIdle()
            assertEquals(
                listOf("setEffort:none"),
                fixture.settings.calls.filter { it.startsWith("setEffort") },
            )
        }

    @Test
    fun a_turn_in_progress_blocks_the_level_choice() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            val vm = model()
            fixture.running = true
            vm.chooseEffort(ThinkingEffort.LOW)
            advanceUntilIdle()
            assertTrue(fixture.settings.calls.none { it.startsWith("setEffort") })
            assertEquals(0, applied)
        }

    @Test
    fun a_model_that_does_not_reason_cannot_take_a_level() =
        runTest(dispatcher) {
            val chat = ModelChoice("chat", "Chat", null, null, null, true, null, false)
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet, chat)
            fixture.settings.session = fixture.settings.session.copy(modelId = "chat")
            val vm = model()
            vm.chooseEffort(ThinkingEffort.HIGH)
            advanceUntilIdle()
            assertTrue(fixture.settings.calls.none { it.startsWith("setEffort") })
            assertEquals(0, fixture.reopenCount)
        }

    @Test
    fun a_busy_core_keeps_the_sheet_open_with_a_failure_on_a_level_choice() =
        runTest(dispatcher) {
            fixture.providers.keys[ProviderView.ANTHROPIC] = "key"
            fixture.settings.modelsByProvider[ProviderView.ANTHROPIC] = listOf(sonnet)
            val vm = model()
            fixture.settings.failNext = SessionSettingsException(SessionFailure.BUSY)
            vm.chooseEffort(ThinkingEffort.HIGH)
            advanceUntilIdle()
            assertEquals(SessionFailure.BUSY, vm.state.value.failure)
            assertEquals(0, fixture.reopenCount)
            assertEquals(0, applied)
        }

    @Test
    fun only_a_model_that_says_it_has_no_tools_cannot_run_agents() {
        assertFalse(ModelChoice("a", "A", null, null, null, false, null, null).canRunAgents)
        assertTrue(ModelChoice("b", "B", null, null, null, null, null, null).canRunAgents)
        assertTrue(sonnet.canRunAgents)
    }
}
