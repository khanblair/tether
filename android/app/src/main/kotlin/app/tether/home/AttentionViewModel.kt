package app.tether.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.updates.UpdatesUiState
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

/**
 * The Needs attention list. It reads the sessions on [io], and it follows [updates], so a newer
 * version appears or leaves the list as the update flow changes. [refresh] reads the sessions
 * again.
 */
class AttentionViewModel(
    private val sessions: SessionsSource,
    updates: StateFlow<UpdatesUiState>,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val sessionList = MutableStateFlow<List<SessionSummary>>(emptyList())

    /** The items that need the user right now. */
    val items: StateFlow<List<AttentionItem>> =
        combine(sessionList, updates) { list, update ->
            attentionItems(list, update)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            attentionItems(emptyList(), updates.value),
        )

    init {
        refresh()
    }

    /** Reads the sessions again. Call it when a screen shows, so new sessions appear. */
    fun refresh() {
        viewModelScope.launch(io) {
            sessionList.value = sessions.sessions()
        }
    }

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
