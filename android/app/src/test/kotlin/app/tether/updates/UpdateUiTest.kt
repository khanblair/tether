package app.tether.updates

import app.tether.R
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class UpdateUiTest {
    private val sample =
        ReleaseSummary(
            version = "0.0.3",
            notes = "Notes",
            publishedAt = "2026-11-20T08:00:00Z",
            apkSizeBytes = 1_000L,
        )

    private fun state(
        phase: UpdatePhase,
        installsInApp: Boolean = true,
        release: ReleaseSummary? = sample,
        apkPath: String? = null,
        problem: UpdateProblem? = null,
    ) = UpdatesUiState(
        currentVersion = "0.0.2",
        installsInApp = installsInApp,
        phase = phase,
        release = release,
        apkPath = apkPath,
        problem = problem,
    )

    @Test
    fun full_build_idle() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_idle_title,
                body = R.string.updates_idle_body,
                primary = UpdateAction.Check,
            ),
            updateCardFor(state(UpdatePhase.Idle, release = null)),
        )
    }

    @Test
    fun full_build_checking() {
        assertEquals(
            UpdateCardUi(title = R.string.updates_checking_title, showSpinner = true),
            updateCardFor(state(UpdatePhase.Checking, release = null)),
        )
    }

    @Test
    fun full_build_up_to_date() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_up_to_date_title,
                body = R.string.updates_up_to_date_body,
                secondary = UpdateAction.Check,
                success = true,
            ),
            updateCardFor(state(UpdatePhase.UpToDate, release = null)),
        )
    }

    @Test
    fun full_build_available() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_available_title,
                primary = UpdateAction.Download,
                showNotes = true,
                showSize = true,
            ),
            updateCardFor(state(UpdatePhase.Available)),
        )
    }

    @Test
    fun full_build_downloading() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_downloading_title,
                secondary = UpdateAction.Cancel,
                showNotes = true,
                showProgress = true,
            ),
            updateCardFor(state(UpdatePhase.Downloading)),
        )
    }

    @Test
    fun full_build_ready() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_ready_title,
                body = R.string.updates_ready_body,
                primary = UpdateAction.Install,
                showNotes = true,
            ),
            updateCardFor(state(UpdatePhase.Ready)),
        )
    }

    @Test
    fun full_build_needs_permission() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_permission_title,
                body = R.string.updates_permission_body,
                primary = UpdateAction.OpenSettings,
                showNotes = true,
            ),
            updateCardFor(state(UpdatePhase.NeedsPermission)),
        )
    }

    @Test
    fun full_build_installing() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_installing_title,
                body = R.string.updates_installing_body,
                showNotes = true,
                showSpinner = true,
            ),
            updateCardFor(state(UpdatePhase.Installing)),
        )
    }

    @Test
    fun full_build_installed() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_installed_title,
                body = R.string.updates_installed_body,
                success = true,
            ),
            updateCardFor(state(UpdatePhase.Installed)),
        )
    }

    @Test
    fun full_build_failed_offers_download_when_release_known() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_failed_title,
                problem = R.string.updates_problem_network,
                primary = UpdateAction.Download,
                showNotes = true,
                failed = true,
            ),
            updateCardFor(state(UpdatePhase.Failed, problem = UpdateProblem.Network)),
        )
    }

    @Test
    fun play_build_idle_checking_and_up_to_date_match_full_build() {
        val play = { phase: UpdatePhase -> state(phase, installsInApp = false, release = null) }
        assertEquals(
            updateCardFor(state(UpdatePhase.Idle, release = null)),
            updateCardFor(play(UpdatePhase.Idle)),
        )
        assertEquals(
            updateCardFor(state(UpdatePhase.Checking, release = null)),
            updateCardFor(play(UpdatePhase.Checking)),
        )
        assertEquals(
            updateCardFor(state(UpdatePhase.UpToDate, release = null)),
            updateCardFor(play(UpdatePhase.UpToDate)),
        )
    }

    @Test
    fun play_build_available_opens_the_store() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_available_title,
                primary = UpdateAction.OpenStore,
                showNotes = true,
            ),
            updateCardFor(state(UpdatePhase.Available, installsInApp = false)),
        )
    }

    @Test
    fun play_build_ready_opens_the_store_instead_of_installing() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_ready_title,
                body = R.string.updates_ready_body,
                primary = UpdateAction.OpenStore,
                showNotes = true,
            ),
            updateCardFor(state(UpdatePhase.Ready, installsInApp = false)),
        )
    }

    @Test
    fun play_build_failed_opens_the_store_when_release_known() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_failed_title,
                problem = R.string.updates_problem_network,
                primary = UpdateAction.OpenStore,
                showNotes = true,
                failed = true,
            ),
            updateCardFor(
                state(UpdatePhase.Failed, installsInApp = false, problem = UpdateProblem.Network),
            ),
        )
    }

    @Test
    fun play_build_failed_without_release_checks_again() {
        assertEquals(
            UpdateCardUi(
                title = R.string.updates_failed_title,
                problem = R.string.updates_problem_network,
                primary = UpdateAction.Check,
                showNotes = true,
                failed = true,
            ),
            updateCardFor(
                state(
                    UpdatePhase.Failed,
                    installsInApp = false,
                    release = null,
                    problem = UpdateProblem.Network,
                ),
            ),
        )
    }

    @Test
    fun play_build_never_offers_download_or_install() {
        val apkPath = "/cache/updates/tether-update.apk"
        UpdatePhase.entries.forEach { phase ->
            val card =
                updateCardFor(
                    state(
                        phase,
                        installsInApp = false,
                        apkPath = apkPath,
                        problem = UpdateProblem.Network,
                    ),
                )
            val offered = listOfNotNull(card.primary, card.secondary)
            assertFalse("$phase offers Download", UpdateAction.Download in offered)
            assertFalse("$phase offers Install", UpdateAction.Install in offered)
        }
    }

    @Test
    fun failed_picks_install_when_apk_is_verified() {
        val card =
            updateCardFor(
                state(
                    UpdatePhase.Failed,
                    apkPath = "/cache/updates/tether-update.apk",
                    problem = UpdateProblem.InstallFailed,
                ),
            )
        assertEquals(UpdateAction.Install, card.primary)
    }

    @Test
    fun failed_picks_download_when_only_release_is_known() {
        val card = updateCardFor(state(UpdatePhase.Failed, problem = UpdateProblem.Network))
        assertEquals(UpdateAction.Download, card.primary)
    }

    @Test
    fun failed_picks_check_otherwise() {
        val card =
            updateCardFor(
                state(UpdatePhase.Failed, release = null, problem = UpdateProblem.Network),
            )
        assertEquals(UpdateAction.Check, card.primary)
    }

    @Test
    fun every_problem_has_its_own_sentence() {
        val sentences = UpdateProblem.entries.map { problemText(it) }
        assertEquals(UpdateProblem.entries.size, sentences.distinct().size)
    }
}
