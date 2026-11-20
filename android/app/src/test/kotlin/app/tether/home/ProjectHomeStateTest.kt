package app.tether.home

import app.tether.design.components.SessionState
import org.junit.Assert.assertEquals
import org.junit.Test

class ProjectHomeStateTest {
    @Test
    fun latestSessionUsesStableProjectIdBeforeARepeatedDisplayName() {
        val project = ProjectSummary("correct", "Same name", "main", 0, 1, false)
        val sessions =
            listOf(
                SessionSummary(
                    "old", "Old", "Same name", SessionState.Done, 2, projectId = "correct",
                ),
                SessionSummary(
                    "new", "New", "Same name", SessionState.Done, 3, projectId = "correct",
                ),
                SessionSummary(
                    "other", "Other", "Same name", SessionState.Done, 4, projectId = "other",
                ),
            )
        assertEquals("new", latestSessionForProject(project, sessions)?.id)
    }
}
