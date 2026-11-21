package app.tether.session

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
class RewindViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val fixture = SheetFixture()
    private var applied = 0
    private var rewound = 0
    private val older = CheckpointItem("old", 1_000L, listOf("a.kt"))
    private val newer = CheckpointItem("new", 5_000L, listOf("a.kt", "b.kt"))

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
        fixture.settings.checkpointList = listOf(older, newer)
    }

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): RewindViewModel {
        val vm =
            RewindViewModel(
                deps = fixture.deps,
                onApplied = { applied++ },
                onRewound = { rewound++ },
                io = dispatcher,
            )
        advanceUntilIdle()
        return vm
    }

    @Test
    fun checkpoints_are_listed_newest_first_and_the_newest_is_selected() =
        runTest(dispatcher) {
            val state = model().state.value
            assertEquals(listOf("new", "old"), state.checkpoints.map { it.id })
            assertEquals("new", state.selected)
            assertFalse(state.loading)
        }

    @Test
    fun a_files_scope_asks_before_it_rewinds() =
        runTest(dispatcher) {
            val vm = model()
            vm.requestRewind()
            advanceUntilIdle()
            assertTrue(vm.state.value.confirming)
            assertTrue(fixture.rewinder.restores.isEmpty())
            vm.confirmRewind()
            advanceUntilIdle()
            assertEquals(
                listOf("new" to RewindScope.FILES_AND_CONVERSATION),
                fixture.rewinder.restores,
            )
            assertEquals(1, rewound)
            assertEquals(1, applied)
        }

    @Test
    fun the_conversation_scope_rewinds_without_asking() =
        runTest(dispatcher) {
            val vm = model()
            vm.setScope(RewindScope.CONVERSATION)
            vm.select("old")
            vm.requestRewind()
            advanceUntilIdle()
            assertFalse(vm.state.value.confirming)
            assertEquals(listOf("old" to RewindScope.CONVERSATION), fixture.rewinder.restores)
        }

    @Test
    fun dismissing_the_confirmation_restores_nothing() =
        runTest(dispatcher) {
            val vm = model()
            vm.requestRewind()
            vm.dismissConfirm()
            advanceUntilIdle()
            assertFalse(vm.state.value.confirming)
            assertTrue(fixture.rewinder.restores.isEmpty())
        }

    @Test
    fun a_corrupt_snapshot_is_kept_as_the_failure_and_the_sheet_stays_open() =
        runTest(dispatcher) {
            val vm = model()
            fixture.rewinder.failNext = SessionSettingsException(SessionFailure.CORRUPT_SNAPSHOT)
            vm.setScope(RewindScope.CONVERSATION)
            vm.requestRewind()
            advanceUntilIdle()
            assertEquals(SessionFailure.CORRUPT_SNAPSHOT, vm.state.value.failure)
            assertEquals(0, applied)
            assertEquals(0, rewound)
        }

    @Test
    fun an_empty_checkpoint_list_has_no_selection_to_rewind() =
        runTest(dispatcher) {
            fixture.settings.checkpointList = emptyList()
            val vm = model()
            assertNull(vm.state.value.selected)
            vm.requestRewind()
            advanceUntilIdle()
            assertTrue(fixture.rewinder.restores.isEmpty())
        }
}
