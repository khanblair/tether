package app.tether.settings

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/** What the privacy screen shows. */
@Immutable
data class PrivacyUiState(
    val appLock: Boolean = false,
)

/**
 * The biometric app lock switch. The screen runs the biometric check before it turns the lock on.
 */
class PrivacyViewModel(
    private val store: SettingsStore,
) : ViewModel() {
    private val mutableState = MutableStateFlow(PrivacyUiState(appLock = store.appLockEnabled()))

    /** The screen state. */
    val state: StateFlow<PrivacyUiState> = mutableState.asStateFlow()

    /** Saves the lock switch. Turning it off needs no check. */
    fun setAppLock(enabled: Boolean) {
        store.saveAppLockEnabled(enabled)
        mutableState.update { it.copy(appLock = enabled) }
    }
}
