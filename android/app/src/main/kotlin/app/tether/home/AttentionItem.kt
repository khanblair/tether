package app.tether.home

import androidx.compose.runtime.Immutable
import app.tether.design.components.SessionState
import app.tether.updates.UpdatesUiState

/** One row on the Needs attention screen. An item leaves the list when its cause is gone. */
@Immutable
sealed interface AttentionItem {
    /** A session is waiting for the user to approve a tool call. */
    @Immutable
    data class Approval(
        val sessionId: String,
        val title: String,
        val projectName: String,
    ) : AttentionItem

    /** A session stopped with an error. */
    @Immutable
    data class Failure(
        val sessionId: String,
        val title: String,
        val projectName: String,
        val updatedAt: Long,
    ) : AttentionItem

    /** A newer app version is out. */
    @Immutable
    data class Update(
        val newVersion: String,
        val currentVersion: String,
    ) : AttentionItem
}

/**
 * The items that need the user, in order: approvals, then failures (both newest first), then the
 * update. [sessions] comes in any order.
 */
fun attentionItems(
    sessions: List<SessionSummary>,
    updates: UpdatesUiState,
): List<AttentionItem> {
    val newestFirst = sessions.sortedByDescending { it.updatedAt }
    val approvals =
        newestFirst
            .filter { it.state == SessionState.Waiting }
            .map { AttentionItem.Approval(it.id, it.title, it.projectName) }
    val failures =
        newestFirst
            .filter { it.state == SessionState.Failed }
            .map { AttentionItem.Failure(it.id, it.title, it.projectName, it.updatedAt) }
    val update =
        updates.newerVersion?.let { AttentionItem.Update(it, updates.currentVersion) }
    return approvals + failures + listOfNotNull(update)
}
