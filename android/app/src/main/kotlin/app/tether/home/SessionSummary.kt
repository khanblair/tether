package app.tether.home

import androidx.compose.runtime.Immutable
import app.tether.design.components.SessionState

/** One agent session as the drawer and Needs attention show it. Times are epoch milliseconds. */
@Immutable
data class SessionSummary(
    val id: String,
    val title: String,
    val projectName: String,
    val state: SessionState,
    val updatedAt: Long,
    val pullRequest: Int? = null,
    val projectId: String? = null,
)
