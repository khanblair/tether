package app.tether.settings

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/** The profile screen. [name] is what the field shows; [savedName] is what is stored. */
@Immutable
data class ProfileUiState(
    val name: String = "",
    val savedName: String = "",
) {
    /** Save is offered only when the trimmed name differs from the saved one. */
    val canSave: Boolean get() = name.trim() != savedName
}

/** Holds the local display name. Nothing about it leaves the phone. */
class ProfileViewModel(
    private val store: SettingsStore,
) : ViewModel() {
    private val mutableState = MutableStateFlow(store.profileName().let { ProfileUiState(it, it) })

    /** The screen state. */
    val state: StateFlow<ProfileUiState> = mutableState.asStateFlow()

    /** The field text changed. */
    fun onNameChange(text: String) {
        mutableState.update { it.copy(name = text) }
    }

    /** Saves the trimmed name. A blank name clears the saved one. */
    fun save() {
        val trimmed = mutableState.value.name.trim()
        store.saveProfileName(trimmed)
        mutableState.update { it.copy(name = trimmed, savedName = trimmed) }
    }
}
