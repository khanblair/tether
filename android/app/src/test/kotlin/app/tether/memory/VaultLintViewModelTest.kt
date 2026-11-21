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
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class VaultLintViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun lint_lists_the_orphan_notes_from_the_core() =
        runTest(dispatcher) {
            val vm =
                VaultLintViewModel(
                    FakeVaultGateway(orphanPaths = listOf("old-deploy.md")),
                    dispatcher,
                )
            advanceUntilIdle()
            assertEquals(listOf("old-deploy.md"), vm.state.value.orphans)
        }

    @Test
    fun a_failed_lint_shows_the_problem() =
        runTest(dispatcher) {
            val gateway = FakeVaultGateway()
            gateway.failure = AreaFailure(AreaProblem.Unavailable)
            val vm = VaultLintViewModel(gateway, dispatcher)
            advanceUntilIdle()
            assertEquals(AreaProblem.Unavailable, vm.state.value.problem)
        }
}
