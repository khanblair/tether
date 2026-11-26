package app.tether.settings

import app.tether.core.generated.ProviderView
import app.tether.home.FakeProjectGateway
import app.tether.home.ProjectInfo
import app.tether.providers.FakeProviderGateway
import app.tether.session.SessionMode
import app.tether.session.ThinkingEffort
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ProfileViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun the_field_starts_with_the_saved_name_and_save_is_off() {
        val vm = ProfileViewModel(FakeSettingsStore(name = "Blair"))

        assertEquals("Blair", vm.state.value.name)
        assertFalse(vm.state.value.canSave)
    }

    @Test
    fun typing_a_new_name_turns_save_on() {
        val vm = ProfileViewModel(FakeSettingsStore(name = "Blair"))

        vm.onNameChange("Blair K")

        assertTrue(vm.state.value.canSave)
    }

    @Test
    fun save_stores_the_trimmed_name_and_turns_save_off() {
        val store = FakeSettingsStore(name = "Blair")
        val vm = ProfileViewModel(store)

        vm.onNameChange("  Blair K  ")
        vm.save()

        assertEquals("Blair K", store.name)
        assertEquals("Blair K", vm.state.value.name)
        assertFalse(vm.state.value.canSave)
    }

    @Test
    fun a_blank_name_clears_the_saved_name() {
        val store = FakeSettingsStore(name = "Blair")
        val vm = ProfileViewModel(store)

        vm.onNameChange("   ")
        vm.save()

        assertEquals("", store.name)
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
class ModelDefaultsViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val anthropicModel = CatalogModel(ProviderView.ANTHROPIC, "sonnet", "Sonnet")
    private val openAiModel = CatalogModel(ProviderView.OPEN_AI, "gpt", "GPT")
    private val providers = FakeProviderGateway()
    private val catalog =
        FakeModelCatalog(
            mapOf(
                ProviderView.ANTHROPIC to listOf(anthropicModel),
                ProviderView.OPEN_AI to listOf(openAiModel),
                ProviderView.GEMINI to listOf(CatalogModel(ProviderView.GEMINI, "g", "G")),
            ),
        )
    private val projects = FakeProjectGateway(listOf(ProjectInfo("p1", "api", 1L)))
    private val defaults = FakeProjectDefaults()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(store: FakeSettingsStore = FakeSettingsStore()) =
        ModelDefaultsViewModel(store, providers, catalog, projects, defaults, io = dispatcher)

    @Test
    fun only_providers_with_a_key_list_their_models() {
        providers.keys[ProviderView.ANTHROPIC] = "key"

        val vm = viewModel()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(
            listOf(ProviderModels(ProviderView.ANTHROPIC, listOf(anthropicModel))),
            vm.state.value.providers,
        )
    }

    @Test
    fun a_provider_with_a_key_and_no_saved_models_is_left_out() {
        providers.keys[ProviderView.OPEN_AI] = "key"
        val vm =
            ModelDefaultsViewModel(
                FakeSettingsStore(),
                providers,
                FakeModelCatalog(),
                projects,
                defaults,
                dispatcher,
            )
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(
            vm.state.value.providers
                .isEmpty(),
        )
        assertFalse(vm.state.value.failed)
    }

    @Test
    fun the_app_effort_starts_at_the_saved_level_and_medium_when_never_set() {
        assertEquals(ThinkingEffort.MEDIUM, viewModel().state.value.effort)
        assertEquals(
            ThinkingEffort.HIGH,
            viewModel(FakeSettingsStore(effort = ThinkingEffort.HIGH)).state.value.effort,
        )
    }

    @Test
    fun choosing_an_app_effort_saves_it_and_closes_the_picker() {
        val store = FakeSettingsStore()
        val vm = viewModel(store)

        vm.openEffort()
        assertTrue(vm.state.value.pickingEffort)
        vm.chooseEffort(ThinkingEffort.HIGH)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ThinkingEffort.HIGH, store.effort)
        assertEquals(ThinkingEffort.HIGH, vm.state.value.effort)
        assertFalse(vm.state.value.pickingEffort)
    }

    @Test
    fun choosing_the_model_default_saves_null() {
        val store = FakeSettingsStore()
        val vm = viewModel(store)

        vm.openEffort()
        vm.chooseEffort(null)
        dispatcher.scheduler.advanceUntilIdle()

        assertNull(store.effort)
        assertNull(vm.state.value.effort)
    }

    @Test
    fun closing_the_effort_picker_changes_nothing() {
        val store = FakeSettingsStore()
        val vm = viewModel(store)

        vm.openEffort()
        vm.close()
        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(vm.state.value.pickingEffort)
        assertEquals(ThinkingEffort.MEDIUM, store.effort)
    }

    @Test
    fun the_app_default_is_saved_and_shown() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        val store = FakeSettingsStore()
        val vm = viewModel(store)
        dispatcher.scheduler.advanceUntilIdle()

        vm.open(ModelTarget.App)
        vm.choose(DefaultModel(ProviderView.ANTHROPIC, "sonnet"))
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(DefaultModel(ProviderView.ANTHROPIC, "sonnet"), store.model)
        assertEquals(DefaultModel(ProviderView.ANTHROPIC, "sonnet"), vm.state.value.appDefault)
        assertNull(vm.state.value.picking)
    }

    @Test
    fun a_project_model_keeps_the_project_mode() {
        defaults.saved["p1"] = ProjectDefaults(null, null, SessionMode.AUTO)
        val vm = viewModel()
        dispatcher.scheduler.advanceUntilIdle()

        vm.open(ModelTarget.Project("p1", "api"))
        vm.choose(DefaultModel(ProviderView.OPEN_AI, "gpt"))
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(
            ProjectDefaults(ProviderView.OPEN_AI, "gpt", SessionMode.AUTO),
            defaults.saved["p1"],
        )
        assertEquals(
            DefaultModel(ProviderView.OPEN_AI, "gpt"),
            vm.state.value.projects
                .single()
                .model,
        )
    }

    @Test
    fun choosing_the_app_default_for_a_project_clears_its_model_only() {
        defaults.saved["p1"] = ProjectDefaults(ProviderView.OPEN_AI, "gpt", SessionMode.ASK)
        val vm = viewModel()
        dispatcher.scheduler.advanceUntilIdle()

        vm.open(ModelTarget.Project("p1", "api"))
        vm.choose(null)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ProjectDefaults(null, null, SessionMode.ASK), defaults.saved["p1"])
        assertNull(
            vm.state.value.projects
                .single()
                .model,
        )
    }

    @Test
    fun a_read_failure_sets_failed_and_empties_the_lists() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        catalog.fail = true

        val vm = viewModel()
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(vm.state.value.failed)
        assertTrue(
            vm.state.value.providers
                .isEmpty(),
        )
    }

    @Test
    fun a_failed_project_save_marks_the_screen_and_keeps_the_old_model() {
        defaults.saved["p1"] = ProjectDefaults(null, null, null)
        val vm = viewModel()
        dispatcher.scheduler.advanceUntilIdle()
        defaults.failLoad = true

        vm.open(ModelTarget.Project("p1", "api"))
        vm.choose(DefaultModel(ProviderView.OPEN_AI, "gpt"))
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(vm.state.value.failed)
        assertNull(
            vm.state.value.projects
                .single()
                .model,
        )
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
class PermissionsViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val defaults = FakeProjectDefaults()
    private val projects = FakeProjectGateway(listOf(ProjectInfo("p1", "api", 1L)))

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(store: FakeSettingsStore = FakeSettingsStore()) =
        PermissionsViewModel(store, projects, defaults, io = dispatcher)

    @Test
    fun the_app_default_mode_is_saved() {
        val store = FakeSettingsStore()
        val vm = viewModel(store)
        dispatcher.scheduler.advanceUntilIdle()

        vm.open(ModeTarget.App)
        vm.choose(SessionMode.ASK)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(SessionMode.ASK, store.mode)
        assertEquals(SessionMode.ASK, vm.state.value.defaultMode)
    }

    @Test
    fun a_project_mode_keeps_its_model() {
        defaults.saved["p1"] = ProjectDefaults(ProviderView.ANTHROPIC, "sonnet", null)
        val vm = viewModel()
        dispatcher.scheduler.advanceUntilIdle()

        vm.open(ModeTarget.Project("p1", "api"))
        vm.choose(SessionMode.PLAN)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(
            ProjectDefaults(ProviderView.ANTHROPIC, "sonnet", SessionMode.PLAN),
            defaults.saved["p1"],
        )
        assertEquals(
            SessionMode.PLAN,
            vm.state.value.projects
                .single()
                .mode,
        )
    }

    @Test
    fun bypass_is_never_saved_as_the_app_default() {
        val store = FakeSettingsStore()
        val vm = viewModel(store)
        dispatcher.scheduler.advanceUntilIdle()

        vm.open(ModeTarget.App)
        vm.choose(SessionMode.BYPASS)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(SessionMode.ACCEPT_EDITS, store.mode)
        assertEquals(SessionMode.ACCEPT_EDITS, vm.state.value.defaultMode)
    }

    @Test
    fun bypass_is_never_saved_for_a_project() {
        defaults.saved["p1"] = ProjectDefaults(null, null, SessionMode.PLAN)
        val vm = viewModel()
        dispatcher.scheduler.advanceUntilIdle()

        vm.open(ModeTarget.Project("p1", "api"))
        vm.choose(SessionMode.BYPASS)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ProjectDefaults(null, null, SessionMode.PLAN), defaults.saved["p1"])
    }

    @Test
    fun a_project_mode_of_null_means_the_app_default() {
        defaults.saved["p1"] = ProjectDefaults(null, null, SessionMode.AUTO)
        val vm = viewModel()
        dispatcher.scheduler.advanceUntilIdle()

        vm.open(ModeTarget.Project("p1", "api"))
        vm.choose(null)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ProjectDefaults(null, null, null), defaults.saved["p1"])
    }
}

class NotificationsViewModelTest {
    @Test
    fun a_channel_with_importance_none_is_off_and_a_missing_one_is_off() {
        val source =
            FakeNotifications(
                importances =
                    mutableMapOf(
                        "approvals" to 4,
                        "run_status" to 0,
                    ),
            )
        val specs = listOf(spec("approvals"), spec("run_status"), spec("budget_errors"))

        val state = NotificationsViewModel(source, specs).state.value

        assertEquals(listOf(true, false, false), state.channels.map { it.on })
    }

    @Test
    fun a_blocked_system_shows_the_off_state_and_offers_the_permission_when_it_can_be_asked() {
        val source = FakeNotifications(enabled = false, canAsk = true)

        val state = NotificationsViewModel(source, emptyList()).state.value

        assertFalse(state.enabled)
        assertTrue(state.canAskForPermission)
    }

    @Test
    fun refresh_reads_a_change_made_in_system_settings() {
        val source = FakeNotifications(enabled = false)
        val vm = NotificationsViewModel(source, listOf(spec("approvals")))

        source.enabled = true
        source.importances["approvals"] = 4
        vm.refresh()

        assertTrue(vm.state.value.enabled)
        assertTrue(
            vm.state.value.channels
                .single()
                .on,
        )
    }

    private fun spec(id: String) = ChannelSpec(id, nameRes = 1, descriptionRes = 2, importance = 0)
}

@OptIn(ExperimentalCoroutinesApi::class)
class DataViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun the_sizes_are_read_on_start() {
        val vm =
            DataViewModel(
                FakeStorage(StorageUsage(1L, 2L, 3L, 4L)),
                onCacheCleared = {},
                io = dispatcher,
            )
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(StorageUsage(1L, 2L, 3L, 4L), vm.state.value.usage)
    }

    @Test
    fun a_clear_that_removes_everything_tells_the_user_and_reads_again() {
        val storage = FakeStorage()
        var cleared = 0
        val vm = DataViewModel(storage, onCacheCleared = { cleared++ }, io = dispatcher)
        dispatcher.scheduler.advanceUntilIdle()

        vm.clearUpdateCache()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(1, storage.clears)
        assertEquals(1, cleared)
        assertFalse(vm.state.value.cacheFailed)
    }

    @Test
    fun a_clear_that_leaves_a_file_behind_shows_the_failure_and_says_nothing_else() {
        val storage = FakeStorage(clearResult = false)
        var cleared = 0
        val vm = DataViewModel(storage, onCacheCleared = { cleared++ }, io = dispatcher)
        dispatcher.scheduler.advanceUntilIdle()

        vm.clearUpdateCache()
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(vm.state.value.cacheFailed)
        assertEquals(0, cleared)
    }
}

class PrivacyViewModelTest {
    @Test
    fun the_switch_starts_from_the_saved_value() {
        val vm = PrivacyViewModel(FakeSettingsStore(lock = true))

        assertTrue(vm.state.value.appLock)
    }

    @Test
    fun setting_the_lock_saves_it() {
        val store = FakeSettingsStore()
        val vm = PrivacyViewModel(store)

        vm.setAppLock(true)

        assertTrue(store.lock)
        assertTrue(vm.state.value.appLock)
    }
}
