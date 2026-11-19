package app.tether.home

import androidx.compose.runtime.Immutable

/** One project in the Projects list. Times are epoch milliseconds. */
@Immutable
data class ProjectSummary(
    val id: String,
    val name: String,
    val branch: String,
    val uncommitted: Int,
    val lastActivityAt: Long,
    val running: Boolean,
)
