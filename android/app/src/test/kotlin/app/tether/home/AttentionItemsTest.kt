package app.tether.home

import app.tether.design.components.SessionState
import app.tether.updates.ReleaseSummary
import app.tether.updates.UpdatesUiState
import org.junit.Assert.assertEquals
import org.junit.Test

class AttentionItemsTest {
    private val sessions =
        listOf(
            SessionSummary("s1", "Migrate to Pydantic v2", "acme-api", SessionState.Waiting, 100L),
            SessionSummary("s2", "Run the test suite", "blog", SessionState.Failed, 300L),
            SessionSummary("s3", "Fix login timeout", "acme-api", SessionState.Running, 50L),
            SessionSummary("s4", "Add UniFFI event path", "tether", SessionState.Done, 400L),
            SessionSummary("s5", "Second approval", "tether", SessionState.Waiting, 200L),
        )

    private fun updates(
        release: ReleaseSummary? = null,
        known: String? = null,
    ) = UpdatesUiState(
        currentVersion = "0.0.3",
        installsInApp = true,
        release = release,
        knownNewerVersion = known,
    )

    private val release = ReleaseSummary("0.0.4", notes = "", publishedAt = "", apkSizeBytes = 0L)

    @Test
    fun every_source_maps_in_order_approvals_then_failures_then_the_update() {
        val items = attentionItems(sessions, updates(release = release))
        assertEquals(
            listOf(
                AttentionItem.Approval("s5", "Second approval", "tether"),
                AttentionItem.Approval("s1", "Migrate to Pydantic v2", "acme-api"),
                AttentionItem.Failure("s2", "Run the test suite", "blog", 300L),
                AttentionItem.Update("0.0.4", "0.0.3"),
            ),
            items,
        )
    }

    @Test
    fun nothing_needing_the_user_gives_an_empty_list() {
        assertEquals(emptyList<AttentionItem>(), attentionItems(emptyList(), updates()))
    }

    @Test
    fun a_running_or_done_session_is_not_listed() {
        val running =
            sessions.filter {
                it.state == SessionState.Running ||
                    it.state == SessionState.Done
            }
        assertEquals(emptyList<AttentionItem>(), attentionItems(running, updates()))
    }

    @Test
    fun the_update_uses_the_release_version() {
        val items = attentionItems(emptyList(), updates(release = release, known = "0.0.5"))
        assertEquals(listOf(AttentionItem.Update("0.0.4", "0.0.3")), items)
    }

    @Test
    fun the_update_falls_back_to_the_known_newer_version() {
        val items = attentionItems(emptyList(), updates(known = "0.0.5"))
        assertEquals(listOf(AttentionItem.Update("0.0.5", "0.0.3")), items)
    }
}
