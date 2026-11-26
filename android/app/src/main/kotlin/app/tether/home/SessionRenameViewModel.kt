package app.tether.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.providers.callCore
import app.tether.session.SessionGateway
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * The rename sheet opened from a list row. [sessionId] is null when no rename is open. [title] is
 * the field text and [currentTitle] is the title the session had when the rename opened.
 */
data class SessionRenameState(
    val sessionId: String? = null,
    val currentTitle: String = "",
    val title: String = "",
    val saving: Boolean = false,
    val failed: Boolean = false,
) {
    /** Save is off while the title is blank or a save is running. */
    val canSave: Boolean
        get() = title.isNotBlank() && !saving
}

/**
 * Renames a session from a list. The title is trimmed, and a blank title is never sent. The core
 * call runs on [io]. On success the state closes and [save] calls its `onRenamed` so the list
 * reloads.
 */
class SessionRenameViewModel(
    private val sessions: SessionGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(SessionRenameState())

    /** The rename state. */
    val state: StateFlow<SessionRenameState> = mutableState.asStateFlow()

    /** Opens the rename for [sessionId], with the field set to its current [title]. */
    fun start(
        sessionId: String,
        title: String,
    ) {
        mutableState.value = SessionRenameState(sessionId, title, title)
    }

    /** The user edited the title field. */
    fun onTitleChange(text: String) {
        mutableState.update { it.copy(title = text, failed = false) }
    }

    /** Saves the trimmed title. Does nothing when no rename is open or the title is blank. */
    fun save(onRenamed: () -> Unit) {
        val current = mutableState.value
        val sessionId = current.sessionId ?: return
        val trimmed = current.title.trim()
        if (trimmed.isEmpty() || current.saving) return
        mutableState.update { it.copy(saving = true, failed = false) }
        viewModelScope.launch {
            callCore(io) { sessions.rename(sessionId, trimmed) }
                .onSuccess {
                    mutableState.value = SessionRenameState()
                    onRenamed()
                }.onFailure {
                    mutableState.update { it.copy(saving = false, failed = true) }
                }
        }
    }

    /** Closes the rename without saving. */
    fun dismiss() {
        mutableState.value = SessionRenameState()
    }

    companion object {
        /** The rename view model over [sessions]. */
        fun factory(sessions: SessionGateway) =
            viewModelFactory { initializer { SessionRenameViewModel(sessions) } }
    }
}
