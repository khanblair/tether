package app.tether.settings.github

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** Where the device flow stands. [Waiting] carries the code and its local expiry time. */
sealed interface DeviceStep {
    /** No device flow is running. */
    data object Idle : DeviceStep

    /** The code is being requested from GitHub. */
    data object Starting : DeviceStep

    /** The code is shown and the app is polling for approval. */
    data class Waiting(
        val grant: DeviceGrant,
        val expiresAtMillis: Long,
    ) : DeviceStep

    /** The user denied access on GitHub. */
    data object Denied : DeviceStep

    /** The code expired before it was approved. */
    data object Expired : DeviceStep
}

/** What the account screen shows. [loading] is true until the first status read finishes. */
data class AccountUiState(
    val loading: Boolean = true,
    val status: GithubStatus = GithubStatus.SignedOut,
    val deviceAvailable: Boolean = true,
    val device: DeviceStep = DeviceStep.Idle,
    val token: String = "",
    val emptyToken: Boolean = false,
    val busy: Boolean = false,
    val failure: GithubFailure? = null,
)

/**
 * State holder for the GitHub account screen. The device flow polls [GithubGateway.pollDeviceFlow]
 * on the timing from [DevicePollTiming]. Polling stops on approval, denial, expiry, a failure,
 * or [stopPolling] when the screen leaves.
 */
class GithubAccountViewModel(
    private val gateway: GithubGateway,
    private val clock: () -> Long = System::currentTimeMillis,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(AccountUiState())

    /** The screen state. */
    val state: StateFlow<AccountUiState> = mutableState.asStateFlow()

    private var pollJob: Job? = null

    init {
        refresh()
    }

    /** Reads the saved account again. */
    fun refresh() {
        viewModelScope.launch {
            call { gateway.status() }
                .onSuccess { status ->
                    mutableState.update {
                        it.copy(
                            loading = false,
                            status = status,
                            failure = null,
                        )
                    }
                }.onFailure { error ->
                    mutableState.update {
                        it.copy(
                            loading = false,
                            failure = error.githubFailure(),
                        )
                    }
                }
        }
    }

    /** The user asked for the device flow: request a code and start polling for approval. */
    fun startDeviceFlow() {
        val current = mutableState.value.device
        if (current is DeviceStep.Starting || current is DeviceStep.Waiting) return
        mutableState.update { it.copy(device = DeviceStep.Starting, failure = null) }
        viewModelScope.launch {
            call { gateway.startDeviceFlow() }
                .onSuccess { grant ->
                    val expiresAt = clock() + grant.expiresInSeconds * MILLIS_PER_SECOND
                    mutableState.update {
                        it.copy(device = DeviceStep.Waiting(grant, expiresAt))
                    }
                    startPolling(grant, expiresAt)
                }.onFailure { error ->
                    val failure = error.githubFailure()
                    if (failure == GithubFailure.NotConfigured) {
                        mutableState.update {
                            it.copy(
                                device = DeviceStep.Idle,
                                deviceAvailable = false,
                            )
                        }
                    } else {
                        mutableState.update { it.copy(device = DeviceStep.Idle, failure = failure) }
                    }
                }
        }
    }

    /** Stops the device flow poll. The screen calls this when it leaves. */
    fun stopPolling() {
        pollJob?.cancel()
        pollJob = null
        if (mutableState.value.device is DeviceStep.Waiting) {
            mutableState.update { it.copy(device = DeviceStep.Idle) }
        }
    }

    /** The user typed in the token field. */
    fun onTokenChange(value: String) {
        mutableState.update { it.copy(token = value, emptyToken = false, failure = null) }
    }

    /** The user pressed Sign in with the typed token. An empty token is refused here. */
    fun signInWithToken() {
        val token = mutableState.value.token.trim()
        if (token.isEmpty()) {
            mutableState.update { it.copy(emptyToken = true) }
            return
        }
        mutableState.update { it.copy(busy = true, failure = null, emptyToken = false) }
        viewModelScope.launch {
            call { gateway.signInWithToken(token) }
                .onSuccess {
                    mutableState.update { it.copy(busy = false, token = "") }
                    refresh()
                }.onFailure { error ->
                    mutableState.update { it.copy(busy = false, failure = error.githubFailure()) }
                }
        }
    }

    /** The user chose Sign out. The token and cached identity are removed. */
    fun signOut() {
        mutableState.update { it.copy(failure = null) }
        viewModelScope.launch {
            call { gateway.signOut() }
                .onSuccess { refresh() }
                .onFailure { error ->
                    mutableState.update { it.copy(failure = error.githubFailure()) }
                }
        }
    }

    override fun onCleared() {
        stopPolling()
    }

    private fun startPolling(
        grant: DeviceGrant,
        expiresAt: Long,
    ) {
        pollJob?.cancel()
        val timing = DevicePollTiming(grant.intervalSeconds, expiresAt, clock)
        pollJob =
            viewModelScope.launch {
                while (true) {
                    delay(timing.delayMillis())
                    if (timing.isExpired()) {
                        finish(DeviceStep.Expired)
                        return@launch
                    }
                    val result = call { gateway.pollDeviceFlow() }
                    val poll =
                        result.getOrElse { error ->
                            finishWithFailure(error.githubFailure())
                            return@launch
                        }
                    when (poll) {
                        GithubSignInPoll.Pending -> Unit
                        GithubSignInPoll.SlowDown -> timing.slowDown()
                        GithubSignInPoll.Authorized -> {
                            finish(DeviceStep.Idle)
                            refresh()
                            return@launch
                        }
                        GithubSignInPoll.Denied -> {
                            finish(DeviceStep.Denied)
                            return@launch
                        }
                        GithubSignInPoll.Expired -> {
                            finish(DeviceStep.Expired)
                            return@launch
                        }
                    }
                }
            }
    }

    private fun finish(step: DeviceStep) {
        pollJob = null
        mutableState.update { it.copy(device = step) }
    }

    private fun finishWithFailure(failure: GithubFailure) {
        pollJob = null
        mutableState.update { it.copy(device = DeviceStep.Idle, failure = failure) }
    }

    private suspend fun <T> call(block: () -> T): Result<T> = callCore(io, block)

    private companion object {
        const val MILLIS_PER_SECOND = 1000L
    }
}
