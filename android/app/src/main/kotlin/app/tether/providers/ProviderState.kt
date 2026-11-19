package app.tether.providers

import androidx.compose.runtime.Immutable
import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderView

/** One provider as the list shows it. */
@Immutable
data class ProviderSummary(
    val provider: ProviderView,
    val hasKey: Boolean = false,
    val status: ConnectionStatusView = ConnectionStatusView.UNSET,
    val modelCount: Int = 0,
    val testedAtMs: Long? = null,
)

/** State of the provider list screen. */
@Immutable
data class ProvidersUiState(
    val items: List<ProviderSummary> = ProviderView.entries.map { ProviderSummary(it) },
    val loading: Boolean = true,
    val failed: Boolean = false,
)

/** What the eye button on the key field does now. */
enum class RevealAction {
    /** Nothing to show: no saved key and nothing typed. The button is hidden. */
    None,

    /** Show the text the user typed or pasted. No check is needed. */
    Show,

    /** Mask the text again. A revealed saved key is also dropped from memory. */
    Hide,

    /** Show the saved key after the system accepts a biometric or screen lock check. */
    Authenticate,
}

/** A problem the detail screen shows as a short message. The screen owns the wording. */
enum class ProviderMessage {
    LoadFailed,
    EmptyKey,
    SaveFailed,
    RemoveFailed,
    RevealFailed,
    TestFailed,
}

/**
 * State of one provider's detail screen. [keyInput] can hold a secret, so [toString] leaves it
 * out and no log line or crash report can carry it.
 */
@Immutable
data class ProviderDetailUiState(
    val provider: ProviderView,
    val loading: Boolean = true,
    val hasKey: Boolean = false,
    val status: ConnectionStatusView = ConnectionStatusView.UNSET,
    val modelCount: Int = 0,
    val testedAtMs: Long? = null,
    val keyInput: String = "",
    val edited: Boolean = false,
    val revealed: Boolean = false,
    val saving: Boolean = false,
    val testing: Boolean = false,
    val testTarget: String = "",
    val message: ProviderMessage? = null,
) {
    /** Save is possible only for a typed key that is not blank. */
    val canSave: Boolean get() = edited && keyInput.isNotBlank() && !saving && !testing

    /** A test needs a saved key and no unsaved edit. */
    val canTest: Boolean get() = hasKey && !edited && !testing && !saving

    /** The action for the eye button, from what the field holds. */
    val revealAction: RevealAction
        get() =
            when {
                revealed -> RevealAction.Hide
                edited && keyInput.isNotEmpty() -> RevealAction.Show
                hasKey && !edited -> RevealAction.Authenticate
                else -> RevealAction.None
            }

    override fun toString(): String =
        "ProviderDetailUiState(provider=$provider, hasKey=$hasKey, status=$status, " +
            "revealed=$revealed, saving=$saving, testing=$testing, message=$message)"
}
