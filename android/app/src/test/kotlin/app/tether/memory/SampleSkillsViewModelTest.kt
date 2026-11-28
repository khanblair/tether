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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class SampleSkillsViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    private fun sample(
        name: String,
        installed: Boolean = false,
    ) = SampleSkillItem(
        name = name,
        description = "Does $name",
        fileCount = 2,
        installed = installed,
    )

    private fun gatewayWith(vararg items: SampleSkillItem) =
        FakeSkillsGateway(
            sampleItems = items.toMutableList(),
            notice = "tdd: MIT, from an open repository.",
        )

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun the_gallery_loads_the_samples_and_the_notice() =
        runTest(dispatcher) {
            val gateway = gatewayWith(sample("tdd"), sample("caveman"))
            val vm = SampleSkillsViewModel(gateway, dispatcher)
            advanceUntilIdle()
            val state = vm.state.value
            assertFalse(state.loading)
            assertEquals(listOf("tdd", "caveman"), state.items.map { it.name })
            assertEquals("tdd: MIT, from an open repository.", state.notice)
            assertNull(state.problem)
        }

    @Test
    fun install_marks_the_sample_installed_after_the_list_is_read_again() =
        runTest(dispatcher) {
            val gateway = gatewayWith(sample("tdd"))
            val vm = SampleSkillsViewModel(gateway, dispatcher)
            advanceUntilIdle()
            vm.install("tdd")
            advanceUntilIdle()
            val state = vm.state.value
            assertEquals(listOf("tdd"), gateway.installs)
            assertTrue(state.items.single().installed)
            assertTrue(state.installing.isEmpty())
            assertNull(state.problem)
        }

    @Test
    fun a_failed_install_sets_the_problem_and_leaves_the_sample_not_installed() =
        runTest(dispatcher) {
            val gateway = gatewayWith(sample("tdd"))
            val vm = SampleSkillsViewModel(gateway, dispatcher)
            advanceUntilIdle()
            gateway.failure = AreaFailure(AreaProblem.Unavailable)
            vm.install("tdd")
            advanceUntilIdle()
            val state = vm.state.value
            assertEquals(AreaProblem.Unavailable, state.problem)
            assertFalse(state.items.single().installed)
            assertTrue(state.installing.isEmpty())
            assertTrue(gateway.installs.isEmpty())
        }

    @Test
    fun installing_the_same_name_twice_calls_the_gateway_once() =
        runTest(dispatcher) {
            val gateway = gatewayWith(sample("tdd"))
            val vm = SampleSkillsViewModel(gateway, dispatcher)
            advanceUntilIdle()
            vm.install("tdd")
            vm.install("tdd")
            advanceUntilIdle()
            vm.install("tdd")
            advanceUntilIdle()
            assertEquals(listOf("tdd"), gateway.installs)
        }

    @Test
    fun refresh_reads_the_samples_again() =
        runTest(dispatcher) {
            val gateway = gatewayWith(sample("tdd"))
            val vm = SampleSkillsViewModel(gateway, dispatcher)
            advanceUntilIdle()
            assertEquals(1, vm.state.value.items.size)
            gateway.sampleItems += sample("caveman")
            vm.refresh()
            advanceUntilIdle()
            val items = vm.state.value.items
            val names = items.map { it.name }
            assertEquals(listOf("tdd", "caveman"), names)
        }
}
