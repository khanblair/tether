package app.tether.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.updates.UpdatesUiState
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn

/**
 * The Needs attention list. It reads the sessions once, and it follows [updates], so a newer
 * version appears or leaves the list as the update flow changes.
 */
class AttentionViewModel(
    sessions: SessionsSource,
    updates: StateFlow<UpdatesUiState>,
) : ViewModel() {
    private val sessionList = sessions.sessions()

    /** The items that need the user right now. */
    val items: StateFlow<List<AttentionItem>> =
        updates
            .map { attentionItems(sessionList, it) }
            .stateIn(
                viewModelScope,
                SharingStarted.Eagerly,
                attentionItems(sessionList, updates.value),
            )

    companion object {
        /** Builds the list model from the app's session source and update flow. */
        fun factory(
            sessions: SessionsSource,
            updates: StateFlow<UpdatesUiState>,
        ) = viewModelFactory {
            initializer { AttentionViewModel(sessions, updates) }
        }
    }
}
