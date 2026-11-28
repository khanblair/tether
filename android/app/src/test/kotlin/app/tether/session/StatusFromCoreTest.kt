package app.tether.session

import app.tether.core.generated.ProviderView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test
import java.io.File

class StatusFromCoreTest {
    private fun details(mode: SessionMode) =
        SessionDetails(
            title = "t",
            provider = ProviderView.ANTHROPIC,
            modelId = "claude-sonnet-5-5",
            mode = mode,
        )

    @Test
    fun status_comes_from_the_details_usage_and_context_window_the_core_reported() {
        val status =
            statusFrom(
                details(SessionMode.AUTO),
                SessionUsage(tokensIn = 1_200, tokensOut = 300, costMicros = 45_000),
                contextWindow = 200_000,
            )

        assertEquals("claude-sonnet-5-5", status.modelId)
        assertEquals("auto", status.mode)
        assertEquals(1_200L, status.tokensIn)
        assertEquals(300L, status.tokensOut)
        assertEquals(45_000L, status.costMicros)
        assertEquals(200_000L, status.contextWindow)
    }

    @Test
    fun an_unknown_context_window_stays_unknown() {
        val status =
            statusFrom(details(SessionMode.ASK), SessionUsage(0, 0, 0), contextWindow = null)

        assertEquals(null, status.contextWindow)
        assertEquals(null, status.contextPercent)
    }

    @Test
    fun every_mode_has_the_name_the_status_line_reads() {
        assertEquals(
            listOf("plan", "ask", "accept_edits", "auto", "bypass"),
            SessionMode.entries.map { it.statusName() },
        )
    }

    /**
     * The app must never open tether.db with Android's SQLite. A second SQLite library on the same
     * WAL database in the same process resets the WAL index the Rust core has mapped, and the
     * core dies with SIGBUS (seen in walFindFrame on 28 Nov 2026).
     */
    @Test
    fun no_app_code_opens_the_database_with_androids_sqlite() {
        val roots =
            listOf("src/main/kotlin", "src/full/kotlin", "src/play/kotlin").map(::File)
        val offenders =
            roots
                .filter { it.exists() }
                .flatMap { root -> root.walkTopDown().filter { it.extension == "kt" }.toList() }
                .filter { file ->
                    val text = file.readText()
                    text.contains("android.database.sqlite") || text.contains("SQLiteOpenHelper")
                }.map { it.path }

        assertFalse("Android SQLite is used in: $offenders", offenders.isNotEmpty())
        assertEquals(emptyList<String>(), offenders)
    }
}
