package app.tether.onboarding

import androidx.compose.runtime.Immutable
import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderView
import app.tether.providers.ProviderMessage

/** Where the add-key step stands. [Tested] means the provider answered with the result. */
enum class AddKeyPhase {
    Idle,
    Saving,
    Testing,
    Tested,
    Failed,
}

/**
 * What the add-key step shows. [keyInput] is the secret, so [toString] leaves it out. [result] is
 * the connection status once the test has answered. [message] names a failure that is not a
 * connection result, such as a key the core could not save.
 */
@Immutable
data class AddKeyUiState(
    val provider: ProviderView = ProviderView.entries.first(),
    val keyInput: String = "",
    val revealed: Boolean = false,
    val phase: AddKeyPhase = AddKeyPhase.Idle,
    val result: ConnectionStatusView = ConnectionStatusView.UNSET,
    val message: ProviderMessage? = null,
) {
    /** True while the key is being saved or tested. The controls wait. */
    val busy: Boolean get() = phase == AddKeyPhase.Saving || phase == AddKeyPhase.Testing

    /** True once the provider accepted the key. The flow then moves on. */
    val accepted: Boolean get() = phase == AddKeyPhase.Tested && result == ConnectionStatusView.OK

    /** Save is possible for a key that is not blank, while nothing is in flight. */
    val canSave: Boolean get() = keyInput.isNotBlank() && !busy && !accepted

    /** The same values with the last result cleared, for an edit that makes it stale. */
    internal fun cleared(): AddKeyUiState =
        copy(phase = AddKeyPhase.Idle, result = ConnectionStatusView.UNSET, message = null)

    override fun toString(): String =
        "AddKeyUiState(provider=$provider, phase=$phase, result=$result, message=$message)"
}
