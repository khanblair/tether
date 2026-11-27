package app.tether.settings.github

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class GithubAccountViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeGithubGateway()
    private var now = 0L
    private lateinit var vm: GithubAccountViewModel

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun createViewModel() {
        vm = GithubAccountViewModel(gateway, clock = { now }, io = dispatcher)
        dispatcher.scheduler.runCurrent()
    }

    /** Moves the fake clock and the test scheduler together, then runs what is due. */
    private fun advance(millis: Long) {
        now += millis
        dispatcher.scheduler.advanceTimeBy(millis)
        dispatcher.scheduler.runCurrent()
    }

    private fun startFlow() {
        vm.startDeviceFlow()
        dispatcher.scheduler.runCurrent()
    }

    @Test
    fun a_pending_poll_keeps_polling_at_the_grant_interval() {
        gateway.polls += listOf(GithubSignInPoll.Pending, GithubSignInPoll.Pending)
        createViewModel()
        startFlow()
        advance(4_999)
        assertEquals(0, gateway.pollCalls)
        advance(1)
        assertEquals(1, gateway.pollCalls)
        advance(5_000)
        assertEquals(2, gateway.pollCalls)
        assertTrue(vm.state.value.device is DeviceStep.Waiting)
    }

    @Test
    fun slow_down_adds_five_seconds_to_the_wait() {
        gateway.polls += listOf(GithubSignInPoll.SlowDown, GithubSignInPoll.Pending)
        createViewModel()
        startFlow()
        advance(5_000)
        assertEquals(1, gateway.pollCalls)
        advance(9_999)
        assertEquals(1, gateway.pollCalls)
        advance(1)
        assertEquals(2, gateway.pollCalls)
    }

    @Test
    fun authorized_stops_polling_and_refreshes_the_account() {
        gateway.polls += GithubSignInPoll.Authorized
        createViewModel()
        gateway.status = GithubStatus(true, "octo", "Octo Cat", "octo@example.com", listOf("repo"))
        startFlow()
        advance(5_000)
        assertEquals(DeviceStep.Idle, vm.state.value.device)
        assertTrue(vm.state.value.status.signedIn)
        assertEquals("octo", vm.state.value.status.login)
        advance(60_000)
        assertEquals(1, gateway.pollCalls)
    }

    @Test
    fun denied_stops_polling_and_shows_the_reason() {
        gateway.polls += GithubSignInPoll.Denied
        createViewModel()
        startFlow()
        advance(5_000)
        assertEquals(DeviceStep.Denied, vm.state.value.device)
        advance(60_000)
        assertEquals(1, gateway.pollCalls)
    }

    @Test
    fun a_code_that_expires_on_github_stops_polling() {
        gateway.polls += GithubSignInPoll.Expired
        createViewModel()
        startFlow()
        advance(5_000)
        assertEquals(DeviceStep.Expired, vm.state.value.device)
        advance(60_000)
        assertEquals(1, gateway.pollCalls)
    }

    @Test
    fun the_local_expiry_time_stops_polling_without_another_poll() {
        gateway.grant = DeviceGrant("ABCD-1234", "https://github.com/login/device", 5, 12)
        createViewModel()
        startFlow()
        advance(10_000)
        assertEquals(2, gateway.pollCalls)
        advance(5_000)
        assertEquals(DeviceStep.Expired, vm.state.value.device)
        assertEquals(2, gateway.pollCalls)
    }

    @Test
    fun a_poll_failure_stops_polling_and_shows_the_failure() {
        gateway.pollFailure = GithubFailure.Network
        createViewModel()
        startFlow()
        advance(5_000)
        assertEquals(DeviceStep.Idle, vm.state.value.device)
        assertEquals(GithubFailure.Network, vm.state.value.failure)
        advance(60_000)
        assertEquals(1, gateway.pollCalls)
    }

    @Test
    fun leaving_the_screen_stops_polling() {
        createViewModel()
        startFlow()
        advance(5_000)
        assertEquals(1, gateway.pollCalls)
        vm.stopPolling()
        advance(60_000)
        assertEquals(1, gateway.pollCalls)
        assertEquals(DeviceStep.Idle, vm.state.value.device)
    }

    @Test
    fun clearing_the_view_model_stops_polling() {
        createViewModel()
        startFlow()
        advance(5_000)
        androidx.lifecycle.ViewModelStore().apply {
            put("k", vm)
            clear()
        }
        advance(60_000)
        assertEquals(1, gateway.pollCalls)
    }

    @Test
    fun a_missing_client_id_hides_the_device_flow() {
        gateway.startFailure = GithubFailure.NotConfigured
        createViewModel()
        startFlow()
        assertFalse(vm.state.value.deviceAvailable)
        assertEquals(DeviceStep.Idle, vm.state.value.device)
        assertNull(vm.state.value.failure)
    }

    @Test
    fun an_empty_token_is_refused_without_a_call() {
        createViewModel()
        vm.onTokenChange("   ")
        vm.signInWithToken()
        dispatcher.scheduler.runCurrent()
        assertTrue(vm.state.value.emptyToken)
        assertTrue(gateway.tokensReceived.isEmpty())
    }

    @Test
    fun a_token_without_the_repo_scope_shows_the_scope_failure() {
        gateway.tokenFailure = GithubFailure.MissingScope
        createViewModel()
        vm.onTokenChange("ghp_example")
        vm.signInWithToken()
        dispatcher.scheduler.runCurrent()
        assertEquals(GithubFailure.MissingScope, vm.state.value.failure)
        assertFalse(vm.state.value.busy)
    }

    @Test
    fun a_valid_token_is_trimmed_sent_and_cleared_from_the_field() {
        createViewModel()
        vm.onTokenChange("  ghp_example  ")
        vm.signInWithToken()
        dispatcher.scheduler.runCurrent()
        assertEquals(listOf("ghp_example"), gateway.tokensReceived)
        assertEquals("", vm.state.value.token)
        assertTrue(vm.state.value.status.signedIn)
        assertEquals(GithubTokenKind.Classic, vm.state.value.status.tokenKind)
        assertTrue(vm.state.value.status.canReadRepos)
    }

    @Test
    fun sign_out_clears_the_account() {
        gateway.status = GithubStatus(true, "octo", null, null, listOf("repo"))
        createViewModel()
        vm.signOut()
        dispatcher.scheduler.runCurrent()
        assertEquals(1, gateway.signOutCalls)
        assertFalse(vm.state.value.status.signedIn)
    }

    @Test
    fun the_saved_account_loads_when_the_screen_opens() {
        gateway.status = GithubStatus(true, "octo", "Octo", null, emptyList())
        createViewModel()
        assertFalse(vm.state.value.loading)
        assertEquals("octo", vm.state.value.status.login)
    }
}
