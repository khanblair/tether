package app.tether.onboarding

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderSettingsException
import app.tether.core.generated.ProviderView
import app.tether.providers.ProviderGateway
import app.tether.providers.ProviderMessage
import app.tether.providers.callCore
import app.tether.providers.connectionMessage
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * State holder for the add-key step. Save and test uses the same gateway calls as the provider
 * detail screen: save the key, then test it. A key the provider accepts calls [onAccepted] once.
 * Every core call runs on [io].
 */
class AddKeyViewModel(
    private val gateway: ProviderGateway,
    private val onAccepted: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(AddKeyUiState())

    /** The screen state. */
    val state: StateFlow<AddKeyUiState> = mutableState.asStateFlow()

    /** Chooses the provider the key is for. Ignored while a save or test runs. */
    fun selectProvider(provider: ProviderView) {
        if (mutableState.value.busy) return
        mutableState.update { it.copy(provider = provider).cleared() }
    }

    /** The user typed or pasted [text] into the key field. A new key clears the last result. */
    fun onKeyChange(text: String) {
        if (mutableState.value.busy) return
        mutableState.update { it.copy(keyInput = text).cleared() }
    }

    /** Masks or shows the typed text. Typed text is not saved yet, so no check is needed. */
    fun toggleReveal() {
        mutableState.update { it.copy(revealed = !it.revealed) }
    }

    /** The screen left the foreground. Typed text is masked again, so it stays off recents. */
    fun onStopped() {
        mutableState.update { it.copy(revealed = false) }
    }

    /** Saves the typed key, clears the field, then tests the saved key with the provider. */
    fun saveAndTest() {
        val typed = mutableState.value
        if (!typed.canSave) return
        mutableState.update { it.copy(phase = AddKeyPhase.Saving, message = null) }
        viewModelScope.launch {
            call { gateway.setKey(typed.provider, typed.keyInput) }
                .onFailure { failure ->
                    fail(saveMessage(failure))
                    return@launch
                }
            mutableState.update {
                it.copy(
                    keyInput = "",
                    revealed = false,
                    phase = AddKeyPhase.Testing,
                )
            }
            call { gateway.testConnection(typed.provider).status }
                .onSuccess(::tested)
                .onFailure { fail(ProviderMessage.TestFailed) }
        }
    }

    /** Keeps the answer. An accepted key calls [onAccepted]. A status with no message fails. */
    private fun tested(status: ConnectionStatusView) {
        if (status != ConnectionStatusView.OK && connectionMessage(status) == null) {
            fail(ProviderMessage.TestFailed)
            return
        }
        mutableState.update {
            it.copy(phase = AddKeyPhase.Tested, result = status, message = null)
        }
        if (status == ConnectionStatusView.OK) onAccepted()
    }

    private fun fail(message: ProviderMessage) {
        mutableState.update { it.copy(phase = AddKeyPhase.Failed, message = message) }
    }

    private fun saveMessage(failure: Throwable): ProviderMessage =
        if (failure is ProviderSettingsException.EmptyKey) {
            ProviderMessage.EmptyKey
        } else {
            ProviderMessage.SaveFailed
        }

    private suspend fun <T> call(block: () -> T): Result<T> = callCore(io, block)

    companion object {
        /** Builds the add-key state holder. [onAccepted] runs when the provider accepts the key. */
        fun factory(
            gateway: ProviderGateway,
            onAccepted: () -> Unit,
        ) = viewModelFactory { initializer { AddKeyViewModel(gateway, onAccepted) } }
    }
}
