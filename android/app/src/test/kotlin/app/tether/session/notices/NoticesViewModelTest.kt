package app.tether.session.notices

import app.tether.session.agent.AgentBanner
import app.tether.session.agent.AgentLoopException
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.BudgetScope
import app.tether.session.agent.FakeAgentLoopGateway
import app.tether.session.agent.InterruptionUi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class NoticesViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeAgentLoopGateway()
    private val banners = MutableStateFlow<List<AgentBanner>>(emptyList())
    private var continued = 0
    private var handled = 0

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    /** Runs the queued coroutines of the test dispatcher until none are left. */
    private fun settle() = dispatcher.scheduler.advanceUntilIdle()

    private fun model(): NoticesViewModel =
        NoticesViewModel(
            sessionId = "s1",
            gateway = gateway,
            banners = banners,
            onBudgetContinued = { continued++ },
            onInterruptionHandled = { handled++ },
            io = dispatcher,
        ).also { settle() }

    private fun interrupted() {
        gateway.interruption = InterruptionUi(step = "npm test", atMs = 5L)
    }

    @Test
    fun interruptionIsLoadedOnce() {
        interrupted()
        val vm = model()
        vm.state.value.let { assertEquals(InterruptionUi("npm test", 5L), it.interruption) }
        settle()
        assertEquals(listOf("interruption:s1"), gateway.log)
    }

    @Test
    fun resumeCallsOnceAndClearsTheBlock() {
        interrupted()
        val vm = model()
        vm.onResume()
        settle()
        assertEquals(listOf("interruption:s1", "resume:s1"), gateway.log)
        assertNull(vm.state.value.interruption)
        assertNull(vm.state.value.running)
        assertNull(vm.state.value.failed)
        assertEquals(1, handled)
    }

    @Test
    fun discardCallsOnceAndClearsTheBlock() {
        interrupted()
        val vm = model()
        vm.onDiscard()
        settle()
        assertEquals(listOf("interruption:s1", "discardInterrupted:s1"), gateway.log)
        assertNull(vm.state.value.interruption)
        assertEquals(1, handled)
    }

    @Test
    fun failedResumeKeepsTheBlockAndSetsTheFailure() {
        interrupted()
        val vm = model()
        gateway.failNext = AgentLoopException(AgentLoopFailure.Busy)
        vm.onResume()
        settle()
        assertEquals(NoticeAction.Resume, vm.state.value.failed)
        assertEquals(InterruptionUi("npm test", 5L), vm.state.value.interruption)
        assertNull(vm.state.value.running)
        assertEquals(0, handled)
    }

    @Test
    fun continueCallsOnceAndFiresTheCallback() {
        val vm = model()
        vm.onContinue()
        settle()
        assertEquals(listOf("interruption:s1", "continuePastBudget:s1"), gateway.log)
        assertEquals(1, continued)
        assertNull(vm.state.value.failed)
    }

    @Test
    fun failedContinueKeepsTheStopAndSetsTheFailure() {
        val vm = model()
        gateway.failNext = AgentLoopException(AgentLoopFailure.Unavailable)
        vm.onContinue()
        settle()
        assertEquals(NoticeAction.Continue, vm.state.value.failed)
        assertEquals(0, continued)
    }

    @Test
    fun bannersAreMirroredIntoTheState() {
        val vm = model()
        banners.value =
            listOf(
                AgentBanner.NetworkPaused,
                AgentBanner.BudgetWarning(BudgetScope.Day, 82, 1_640_000L, 2_000_000L),
                AgentBanner.BudgetReached(BudgetScope.Session, 2_000_000L, 2_000_000L),
            )
        settle()
        val state = vm.state.value
        assertEquals(true, state.networkPaused)
        assertEquals(1, state.warnings.size)
        assertEquals(1, state.reached.size)
    }
}
