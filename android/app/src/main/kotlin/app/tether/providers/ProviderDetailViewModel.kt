package app.tether.providers

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderSettingsException
import app.tether.core.generated.ProviderStatusView
import app.tether.core.generated.ProviderView
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** State holder for one provider's key, test, and status. Every core call runs on [io]. */
class ProviderDetailViewModel(
    private val provider: ProviderView,
    private val gateway: ProviderGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(ProviderDetailUiState(provider))

    /** The screen state. */
    val state: StateFlow<ProviderDetailUiState> = mutableState.asStateFlow()

    init {
        load()
    }

    /** Reads the saved key flag, status, and test target. */
    fun load() {
        viewModelScope.launch {
            call {
                Triple(
                    gateway.hasKey(provider),
                    gateway.status(provider),
                    gateway.testTarget(provider),
                )
            }.onSuccess { (hasKey, saved, target) ->
                mutableState.update {
                    it
                        .withSaved(
                            hasKey,
                            saved,
                        ).copy(loading = false, testTarget = target)
                }
            }.onFailure {
                mutableState.update {
                    it.copy(
                        loading = false,
                        message = ProviderMessage.LoadFailed,
                    )
                }
            }
        }
    }

    /** The user typed or pasted [text] into the key field. */
    fun onKeyChange(text: String) {
        // A save in flight sends the text it started with, then clears the field. An edit made
        // now would be cleared unsaved, so it is ignored. The screen also disables the field.
        mutableState.update {
            if (it.saving) it else it.copy(keyInput = text, edited = true, message = null)
        }
    }

    /** Saves the typed key. The core trims it and rejects an empty one. */
    fun save() {
        val typed = mutableState.value
        if (!typed.canSave) return
        mutableState.update { it.copy(saving = true, message = null) }
        viewModelScope.launch {
            call {
                gateway.setKey(provider, typed.keyInput)
                gateway.status(provider)
            }.onSuccess { saved ->
                mutableState.update { it.withSaved(true, saved).cleared() }
            }.onFailure { failure ->
                val message =
                    if (failure is ProviderSettingsException.EmptyKey) {
                        ProviderMessage.EmptyKey
                    } else {
                        ProviderMessage.SaveFailed
                    }
                mutableState.update { it.copy(saving = false, message = message) }
            }
        }
    }

    /** Removes the saved key, its status, and its cached model list. */
    fun remove() {
        if (mutableState.value.saving || mutableState.value.testing) return
        mutableState.update { it.copy(saving = true, message = null) }
        viewModelScope.launch {
            call { gateway.removeKey(provider) }
                .onSuccess {
                    mutableState.update {
                        it
                            .copy(
                                hasKey = false,
                                status = ConnectionStatusView.UNSET,
                                modelCount = 0,
                                testedAtMs = null,
                            ).cleared()
                    }
                }.onFailure {
                    mutableState.update {
                        it.copy(
                            saving = false,
                            message = ProviderMessage.RemoveFailed,
                        )
                    }
                }
        }
    }

    /** Tests the saved key against the provider and keeps the result. */
    fun test() {
        if (!mutableState.value.canTest) return
        mutableState.update { it.copy(testing = true, message = null) }
        viewModelScope.launch {
            call { gateway.testConnection(provider) }
                .onSuccess { saved ->
                    mutableState.update { it.withSaved(true, saved).copy(testing = false) }
                }.onFailure {
                    mutableState.update {
                        it.copy(
                            testing = false,
                            message = ProviderMessage.TestFailed,
                        )
                    }
                }
        }
    }

    /** Call this only after the system accepted a biometric check. Shows the saved key. */
    fun reveal() {
        viewModelScope.launch {
            call { gateway.revealKey(provider) }
                .onSuccess { key ->
                    if (key == null) {
                        mutableState.update { it.copy(message = ProviderMessage.RevealFailed) }
                    } else {
                        mutableState.update {
                            it.copy(
                                keyInput = key,
                                revealed = true,
                                edited = false,
                                message = null,
                            )
                        }
                    }
                }.onFailure {
                    mutableState.update {
                        it.copy(
                            message = ProviderMessage.RevealFailed,
                        )
                    }
                }
        }
    }

    /** Shows the text the user typed or pasted. A saved key needs [reveal] after a check. */
    fun show() {
        mutableState.update { if (it.edited) it.copy(revealed = true) else it }
    }

    /** Masks the field. A revealed saved key is also dropped from memory; typed text is kept. */
    fun hide() {
        mutableState.update {
            if (it.edited) it.copy(revealed = false) else it.copy(keyInput = "", revealed = false)
        }
    }

    /** The screen left the foreground. A revealed key must not stay on screen. */
    fun onStopped() {
        if (mutableState.value.revealed) hide()
    }

    /** The user closed the message. */
    fun dismissMessage() {
        mutableState.update { it.copy(message = null) }
    }

    private suspend fun <T> call(block: () -> T): Result<T> =
        try {
            Result.success(withContext(io) { block() })
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (
            @Suppress("TooGenericExceptionCaught") failure: Exception,
        ) {
            Result.failure(failure)
        }

    companion object {
        /** Builds the detail model for [provider] with the app's gateway. */
        fun factory(
            provider: ProviderView,
            gateway: ProviderGateway,
        ) = viewModelFactory { initializer { ProviderDetailViewModel(provider, gateway) } }
    }
}

private fun ProviderDetailUiState.withSaved(
    hasKey: Boolean,
    saved: ProviderStatusView,
): ProviderDetailUiState =
    copy(
        hasKey = hasKey,
        status = if (hasKey) saved.status else ConnectionStatusView.UNSET,
        modelCount = saved.modelCount.toInt(),
        testedAtMs = saved.testedAtMs,
    )

private fun ProviderDetailUiState.cleared(): ProviderDetailUiState =
    copy(keyInput = "", edited = false, revealed = false, saving = false)
