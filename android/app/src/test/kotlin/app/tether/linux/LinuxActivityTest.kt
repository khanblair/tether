package app.tether.linux

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LinuxActivityTest {
    @Test
    fun desired_service_covers_every_activity_table_cell() {
        val cases =
            LinuxBackgroundMode.entries.flatMap { mode ->
                activityStates().map { it to mode }
            }
        cases.forEach { (state, mode) ->
            assertEquals(expectedAction(state, mode), desiredService(state, mode))
        }
    }

    @Test
    fun background_store_defaults_round_trips_and_rejects_unknown_values() {
        var saved: String? = null
        val store = LinuxBackgroundStore(read = { saved }, write = { saved = it })

        assertEquals(LinuxBackgroundMode.WhileRunning, store.load())

        store.save(LinuxBackgroundMode.WhenInstalled)
        assertEquals(LinuxBackgroundMode.WhenInstalled, store.load())

        saved = "not-a-mode"
        assertEquals(LinuxBackgroundMode.WhileRunning, store.load())
    }

    @Test
    fun tracker_reports_a_shell_and_drops_it_when_closed() {
        val service = FakeLinuxServiceController()
        val tracker = LinuxActivity(LinuxBackgroundController(emptyStore()), service)
        tracker.setAppVisible(true)

        tracker.openSource("shell")
        tracker.reportShell("shell", true)
        assertEquals(run(LinuxNoticeKind.ShellOpen), service.actions.last())

        tracker.removeSource("shell")
        assertEquals(ServiceAction.Stop, service.actions.last())
    }

    @Test
    fun tracker_sums_two_sources_and_ignores_a_late_report_from_a_closed_source() {
        val service = FakeLinuxServiceController()
        val tracker = LinuxActivity(LinuxBackgroundController(emptyStore()), service)
        tracker.setAppVisible(true)

        tracker.openSource("one")
        tracker.openSource("two")
        tracker.reportServers("one", 1)
        tracker.reportServers("two", 2)
        assertEquals(run(LinuxNoticeKind.Servers, 3), service.actions.last())

        tracker.removeSource("one")
        assertEquals(run(LinuxNoticeKind.Servers, 2), service.actions.last())
        val actionCount = service.actions.size
        tracker.reportServers("one", 3)

        assertEquals(actionCount, service.actions.size)
        assertEquals(run(LinuxNoticeKind.Servers, 2), service.actions.last())
    }

    @Test
    fun mode_switch_starts_and_stops_when_linux_is_ready_but_idle() {
        val service = FakeLinuxServiceController()
        val settings = LinuxBackgroundController(emptyStore())
        val tracker = LinuxActivity(settings, service)
        tracker.setAppVisible(true)
        tracker.reportLinuxReady(true)

        settings.set(LinuxBackgroundMode.WhenInstalled)
        assertEquals(run(LinuxNoticeKind.Ready), service.actions.last())

        settings.set(LinuxBackgroundMode.WhileRunning)
        assertEquals(ServiceAction.Stop, service.actions.last())
    }

    @Test
    fun repeated_identical_state_does_not_restart_the_service() {
        val service = FakeLinuxServiceController()
        val tracker = LinuxActivity(LinuxBackgroundController(emptyStore()), service)
        tracker.setAppVisible(true)

        tracker.reportServers("session", 1)
        tracker.reportServers("session", 1)

        assertEquals(listOf(run(LinuxNoticeKind.Servers, 1)), service.actions)
        assertTrue(service.actions.single() is ServiceAction.Run)
    }

    @Test
    fun notice_round_trips_through_its_extras_and_rejects_unknown_kinds() {
        LinuxNoticeKind.entries.forEach { kind ->
            val notice = LinuxNotice(kind, 2)
            assertEquals(notice, LinuxNotice.fromExtras(kind.name, 2))
        }
        assertEquals(null, LinuxNotice.fromExtras("Nope", 1))
        assertEquals(null, LinuxNotice.fromExtras(null, 1))
        assertEquals(0, LinuxNotice.fromExtras("Servers", -4)?.servers)
    }

    private fun expectedAction(
        state: LinuxActivityState,
        mode: LinuxBackgroundMode,
    ): ServiceAction {
        val active = state.shellOpen || state.runningProcesses > 0
        val notice =
            when {
                state.shellOpen && state.runningProcesses > 0 ->
                    LinuxNotice(LinuxNoticeKind.ShellAndServers, state.runningProcesses)
                state.runningProcesses > 0 ->
                    LinuxNotice(LinuxNoticeKind.Servers, state.runningProcesses)
                mode == LinuxBackgroundMode.WhenInstalled -> LinuxNotice(LinuxNoticeKind.Ready)
                else -> LinuxNotice(LinuxNoticeKind.ShellOpen)
            }
        return when {
            mode == LinuxBackgroundMode.WhileRunning && !active -> ServiceAction.Stop
            mode == LinuxBackgroundMode.WhenInstalled && !state.linuxReady -> ServiceAction.Stop
            else -> ServiceAction.Run(notice)
        }
    }

    private fun activityStates(): List<LinuxActivityState> =
        listOf(false, true).flatMap { shellOpen ->
            (0..3).flatMap { runningProcesses ->
                listOf(false, true).map { linuxReady ->
                    LinuxActivityState(shellOpen, runningProcesses, linuxReady)
                }
            }
        }

    private fun run(
        kind: LinuxNoticeKind,
        servers: Int = 0,
    ) = ServiceAction.Run(LinuxNotice(kind, servers))

    private fun emptyStore() = LinuxBackgroundStore(read = { null }, write = {})

    private class FakeLinuxServiceController : LinuxServiceController {
        val actions = mutableListOf<ServiceAction>()

        override fun apply(action: ServiceAction): Boolean {
            actions += action
            return true
        }
    }
}
