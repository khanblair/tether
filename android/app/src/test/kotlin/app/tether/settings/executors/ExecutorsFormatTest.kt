package app.tether.settings.executors

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ExecutorsFormatTest {
    private val manifestUrl = "https://releases.example.test/packs/manifest.json"

    @Test
    fun sizes_round_to_whole_megabytes() {
        assertEquals(0L, megabytes(0L))
        assertEquals(620L, megabytes(620_000_000L))
        assertEquals(1L, megabytes(1_499_999L))
        assertEquals(2L, megabytes(1_500_000L))
    }

    @Test
    fun any_size_above_zero_shows_at_least_one_megabyte() {
        assertEquals(1L, megabytes(1L))
        assertEquals(1L, megabytes(499_999L))
    }

    @Test
    fun a_negative_size_shows_zero_megabytes() {
        assertEquals(0L, megabytes(-5L))
    }

    @Test
    fun download_fraction_is_zero_for_an_unknown_total_and_capped_at_one() {
        assertEquals(0f, downloadFraction(downloaded = 50L, total = 0L), 0f)
        assertEquals(0.5f, downloadFraction(downloaded = 50L, total = 100L), 0.0001f)
        assertEquals(1f, downloadFraction(downloaded = 200L, total = 100L), 0f)
        assertEquals(0f, downloadFraction(downloaded = -5L, total = 100L), 0f)
    }

    @Test
    fun durations_split_into_hours_minutes_and_seconds() {
        assertEquals(DurationParts(0L, 0L, 0L), durationParts(0L))
        assertEquals(DurationParts(0L, 0L, 12L), durationParts(12L))
        assertEquals(DurationParts(0L, 4L, 8L), durationParts(248L))
        assertEquals(DurationParts(1L, 2L, 5L), durationParts(3_725L))
    }

    @Test
    fun the_host_of_a_manifest_url_is_shown_and_a_missing_url_gives_null() {
        assertEquals(
            "releases.example.test",
            manifestHost(manifestUrl),
        )
        assertEquals(null, manifestHost(null))
        assertEquals(null, manifestHost("not a url"))
    }

    @Test
    fun a_negative_duration_counts_as_zero() {
        assertEquals(DurationParts(0L, 0L, 0L), durationParts(-3L))
    }

    @Test
    fun an_iso_start_time_parses_to_epoch_milliseconds() {
        val parsed = parseStartedAt("2026-11-22T10:00:00Z")

        assertEquals(1_795_341_600_000L, parsed)
    }

    @Test
    fun a_start_time_that_is_not_iso_gives_null() {
        assertNull(parseStartedAt("yesterday"))
        assertNull(parseStartedAt(""))
    }

    @Test
    fun the_setup_phase_follows_the_byte_counts() {
        assertEquals(LinuxPhase.Starting, linuxPhaseOf(0L, 0L))
        assertEquals(LinuxPhase.Downloading, linuxPhaseOf(0L, 100L))
        assertEquals(LinuxPhase.Downloading, linuxPhaseOf(50L, 100L))
        assertEquals(LinuxPhase.SettingUp, linuxPhaseOf(100L, 100L))
        assertEquals(LinuxPhase.SettingUp, linuxPhaseOf(150L, 100L))
        assertEquals(LinuxPhase.Downloading, linuxPhaseOf(5L, 0L))
    }

    @Test
    fun the_packs_show_ready_only_when_the_core_lists_them_as_installed() {
        val state =
            ExecutorsUiState(
                packsLoaded = true,
                packCore = listOf(pack("a", installed = true), pack("b", installed = false)),
                packInstallAvailable = true,
            )

        val rows = state.packRows().associateBy { it.id }

        assertEquals(PackState.Ready("1.0.0"), rows.getValue("a").state)
        assertEquals(PackState.NotInstalled, rows.getValue("b").state)
    }

    @Test
    fun a_pack_with_no_install_in_this_build_shows_not_available() {
        val state =
            ExecutorsUiState(
                packsLoaded = true,
                packCore = listOf(pack("b", installed = false)),
                packInstallAvailable = false,
            )

        assertEquals(PackState.NotAvailable, state.packRows().single().state)
    }

    @Test
    fun an_override_for_an_install_in_progress_wins_over_the_core_list() {
        val state =
            ExecutorsUiState(
                packsLoaded = true,
                packCore = listOf(pack("b", installed = false)),
                packOverrides = mapOf("b" to PackState.Downloading(10L, 40L)),
                packInstallAvailable = true,
            )

        assertEquals(PackState.Downloading(10L, 40L), state.packRows().single().state)
    }
}
