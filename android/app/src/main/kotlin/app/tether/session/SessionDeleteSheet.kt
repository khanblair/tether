package app.tether.session

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TextButtonTone
import app.tether.design.tokens.TetherSpace
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** The delete sheet's state. [title] is null until it has loaded, and Delete waits for that. */
data class SessionDeleteState(
    val title: String? = null,
    val deleting: Boolean = false,
    val failure: SessionFailure? = null,
) {
    val canDelete: Boolean
        get() = title != null && !deleting

    /** The session's name for the sheet: its title, or [untitled] when it has none. */
    fun name(untitled: String): String = sessionTitleOr(title.orEmpty(), untitled)
}

/**
 * Loads the session's title for the confirmation, and deletes the session on request. [onDeleted]
 * runs after the core has removed it. Delete is refused by the core while a turn runs, and that
 * shows as a failure.
 */
class SessionDeleteViewModel(
    private val deps: SessionSheetDeps,
    private val onDeleted: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(SessionDeleteState())

    /** The screen state. */
    val state: StateFlow<SessionDeleteState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            sessionCall(io) { deps.settings.details(deps.sessionId).title }
                .onSuccess { title -> mutableState.update { it.copy(title = title) } }
                .onFailure { failure ->
                    mutableState.update { it.copy(failure = failure.sessionFailure()) }
                }
        }
    }

    /** Deletes the session. A second tap while one runs does nothing. */
    fun delete() {
        if (!mutableState.value.canDelete) return
        mutableState.update { it.copy(deleting = true, failure = null) }
        viewModelScope.launch {
            sessionCall(io) { deps.settings.delete(deps.sessionId) }
                .onSuccess { onDeleted() }
                .onFailure { failure ->
                    mutableState.update {
                        it.copy(deleting = false, failure = failure.sessionFailure())
                    }
                }
        }
    }

    companion object {
        /** The delete sheet's view model, for the session described by [deps]. */
        fun factory(
            deps: SessionSheetDeps,
            onDeleted: () -> Unit,
        ) = viewModelFactory { initializer { SessionDeleteViewModel(deps, onDeleted) } }
    }
}

/** The confirmation before a session is deleted. It names the session and says it is final. */
@Composable
fun SessionDeleteSheet(
    state: SessionDeleteState,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    val name = state.name(stringResource(R.string.session_untitled))
    TetherSheet(
        title = stringResource(R.string.session_delete_title, name),
        onDismiss = onDismiss,
    ) {
        Column(
            modifier = Modifier.padding(horizontal = TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            Text(
                text = stringResource(R.string.session_delete_body),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
            if (state.failure != null) {
                InlineBanner(text = deleteFailureText(state.failure), tone = BannerTone.Danger)
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
            ) {
                TetherTextButton(
                    label = stringResource(R.string.session_delete_cancel),
                    onClick = onDismiss,
                )
                TetherTextButton(
                    label = stringResource(R.string.session_delete_confirm),
                    onClick = onConfirm,
                    tone = TextButtonTone.Danger,
                    enabled = state.canDelete,
                )
            }
        }
    }
}

@Composable
private fun deleteFailureText(failure: SessionFailure): String =
    if (failure == SessionFailure.BUSY) {
        stringResource(R.string.session_menu_busy)
    } else {
        stringResource(R.string.session_error_settings)
    }
