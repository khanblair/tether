package app.tether.memory

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class VaultListViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun a_blank_query_shows_the_index_and_the_log_when_they_exist() =
        runTest(dispatcher) {
            val notes =
                mutableMapOf(
                    "log.md" to note("log.md", "log"),
                    "index.md" to note("index.md", "index"),
                )
            val vm = VaultListViewModel(FakeVaultGateway(notes), dispatcher)
            advanceUntilIdle()
            assertEquals(
                listOf("index.md", "log.md"),
                vm.state.value.notes
                    .map { it.path },
            )
            assertFalse(vm.state.value.loading)
        }

    @Test
    fun a_missing_standard_note_is_skipped_without_an_error() =
        runTest(dispatcher) {
            val vm =
                VaultListViewModel(
                    FakeVaultGateway(
                        mutableMapOf(
                            "log.md" to note("log.md", "log"),
                        ),
                    ),
                    dispatcher,
                )
            advanceUntilIdle()
            assertEquals(
                listOf("log.md"),
                vm.state.value.notes
                    .map { it.path },
            )
            assertEquals(null, vm.state.value.problem)
        }

    @Test
    fun a_query_shows_the_search_hits() =
        runTest(dispatcher) {
            val notes =
                mutableMapOf(
                    "notes/auth.md" to note("notes/auth.md", "x"),
                    "notes/other.md" to note("notes/other.md", "y"),
                )
            val vm = VaultListViewModel(FakeVaultGateway(notes), dispatcher)
            advanceUntilIdle()
            vm.onQueryChange("auth")
            advanceUntilIdle()
            assertEquals(
                listOf("notes/auth.md"),
                vm.state.value.notes
                    .map { it.path },
            )
            assertEquals("auth", vm.state.value.query)
        }

    @Test
    fun a_query_with_no_hits_leaves_the_list_empty_without_a_problem() =
        runTest(dispatcher) {
            val vm =
                VaultListViewModel(
                    FakeVaultGateway(
                        mutableMapOf(
                            "log.md" to note("log.md", "log"),
                        ),
                    ),
                    dispatcher,
                )
            advanceUntilIdle()
            vm.onQueryChange("zzz")
            advanceUntilIdle()
            assertEquals(emptyList<VaultNote>(), vm.state.value.notes)
            assertEquals(null, vm.state.value.problem)
        }

    @Test
    fun refresh_shows_the_new_rows_for_the_same_query() =
        runTest(dispatcher) {
            val gateway =
                FakeVaultGateway(
                    mutableMapOf(
                        "notes/auth.md" to note("notes/auth.md", "x"),
                    ),
                )
            val vm = VaultListViewModel(gateway, dispatcher)
            advanceUntilIdle()
            vm.onQueryChange("auth")
            advanceUntilIdle()
            gateway.notes["notes/auth-two.md"] = note("notes/auth-two.md", "y")
            vm.refresh()
            advanceUntilIdle()
            assertEquals(
                listOf("notes/auth.md", "notes/auth-two.md"),
                vm.state.value.notes
                    .map { it.path },
            )
            assertEquals("auth", vm.state.value.query)
        }

    @Test
    fun refresh_with_a_blank_query_shows_a_standard_note_created_since() =
        runTest(dispatcher) {
            val gateway = FakeVaultGateway()
            val vm = VaultListViewModel(gateway, dispatcher)
            advanceUntilIdle()
            assertEquals(emptyList<VaultNote>(), vm.state.value.notes)
            gateway.notes["index.md"] = note("index.md", "index")
            vm.refresh()
            advanceUntilIdle()
            assertEquals(
                listOf("index.md"),
                vm.state.value.notes
                    .map { it.path },
            )
        }

    @Test
    fun a_failed_search_shows_the_problem() =
        runTest(dispatcher) {
            val gateway = FakeVaultGateway(mutableMapOf("log.md" to note("log.md", "log")))
            val vm = VaultListViewModel(gateway, dispatcher)
            advanceUntilIdle()
            gateway.failure = AreaFailure(AreaProblem.Unavailable)
            vm.onQueryChange("log")
            advanceUntilIdle()
            assertEquals(AreaProblem.Unavailable, vm.state.value.problem)
            assertEquals(emptyList<VaultNote>(), vm.state.value.notes)
        }
}
