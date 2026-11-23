package app.tether.settings.executors

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ExecutorsViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    private val build =
        ExecutorBuildConfig(
            packManifestUrl = "https://example.test/packs.json",
            rootfsManifestUrl = "https://example.test/rootfs.json",
            linuxSupported = true,
        )

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(
        gateway: FakeExecutorsGateway,
        config: ExecutorBuildConfig = build,
        projectId: String? = "tether",
    ): ExecutorsViewModel =
        ExecutorsViewModel(gateway, config, projectId, dispatcher).also {
            dispatcher.scheduler.advanceUntilIdle()
        }

    private fun ExecutorsViewModel.rowState(id: String): PackState? =
        state.value
            .packRows()
            .firstOrNull { it.id == id }
            ?.state

    @Test
    fun an_installed_pack_shows_ready_with_its_version() {
        val gateway = FakeExecutorsGateway(packs = listOf(pack("python-3.12", installed = true)))

        val vm = viewModel(gateway)

        assertEquals(PackState.Ready("1.0.0"), vm.rowState("python-3.12"))
    }

    @Test
    fun a_listed_pack_that_is_not_installed_offers_install() {
        val gateway = FakeExecutorsGateway(packs = listOf(pack("quickjs", installed = false)))

        val vm = viewModel(gateway)

        assertEquals(PackState.NotInstalled, vm.rowState("quickjs"))
    }

    @Test
    fun an_empty_pack_list_is_loaded_and_has_no_rows() {
        val vm = viewModel(FakeExecutorsGateway())

        assertTrue(vm.state.value.packsLoaded)
        assertTrue(
            vm.state.value
                .packRows()
                .isEmpty(),
        )
        assertEquals(null, vm.state.value.packsFailure)
    }

    @Test
    fun an_install_moves_through_downloading_and_verifying_to_ready() {
        val gateway =
            FakeExecutorsGateway(
                packs = listOf(pack("python-3.12", installed = false)),
                installProgress = listOf(10L to 40L, 40L to 40L, 40L to 40L),
            )
        val vm = viewModel(gateway)
        val seen = mutableListOf<PackState?>()
        gateway.beforeReport = { seen += vm.rowState("python-3.12") }

        vm.installPack("python-3.12")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(
            listOf<PackState?>(
                PackState.Downloading(0L, 0L),
                PackState.Downloading(10L, 40L),
                PackState.Verifying,
            ),
            seen,
        )
        assertEquals(PackState.Ready("1.0.0"), vm.rowState("python-3.12"))
    }

    @Test
    fun a_failed_install_shows_the_failure_and_try_again_runs_it_again() {
        val gateway =
            FakeExecutorsGateway(
                packs = listOf(pack("node-22", installed = false)),
                installFailure = ExecutorsFailure.VerificationFailed,
            )
        val vm = viewModel(gateway)

        vm.installPack("node-22")
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(PackState.Failed(ExecutorsFailure.VerificationFailed), vm.rowState("node-22"))

        gateway.installFailure = null
        vm.installPack("node-22")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(PackState.Ready("1.0.0"), vm.rowState("node-22"))
        assertEquals(
            listOf("install:node-22", "install:node-22"),
            gateway.calls.filter {
                it.startsWith("install")
            },
        )
    }

    @Test
    fun an_install_the_build_cannot_configure_shows_not_available() {
        val gateway =
            FakeExecutorsGateway(
                packs = listOf(pack("go-1.24", installed = false)),
                installFailure = ExecutorsFailure.NotConfigured,
            )
        val vm = viewModel(gateway)

        vm.installPack("go-1.24")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(PackState.NotAvailable, vm.rowState("go-1.24"))
    }

    @Test
    fun a_build_without_a_pack_manifest_never_calls_install() {
        val gateway = FakeExecutorsGateway(packs = listOf(pack("quickjs", installed = false)))
        val vm = viewModel(gateway, config = build.copy(packManifestUrl = null))

        vm.installPack("quickjs")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(PackState.NotAvailable, vm.rowState("quickjs"))
        assertFalse(gateway.calls.any { it.startsWith("install") })
    }

    @Test
    fun cancel_returns_the_row_to_not_installed_and_reads_the_list_again() {
        val gateway =
            FakeExecutorsGateway(
                packs = listOf(pack("sqlite", installed = false)),
                installProgress = listOf(5L to 40L),
            )
        val vm = viewModel(gateway)
        var afterCancel: PackState? = null
        gateway.beforeReport = {
            vm.cancelPackInstall("sqlite")
            afterCancel = vm.rowState("sqlite")
        }

        vm.installPack("sqlite")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(PackState.Detached, afterCancel)
        assertEquals(PackState.Ready("1.0.0"), vm.rowState("sqlite"))
        assertEquals(2, gateway.calls.count { it == "runtimePacks" })
    }

    @Test
    fun a_detached_download_blocks_a_second_install_until_the_core_run_ends() {
        val gateway =
            FakeExecutorsGateway(
                packs = listOf(pack("sqlite", installed = false)),
                installProgress = listOf(5L to 40L),
            )
        val vm = viewModel(gateway)
        var reports = 0
        gateway.beforeReport = {
            reports++
            if (reports == 1) {
                vm.cancelPackInstall("sqlite")
                vm.installPack("sqlite")
            }
        }

        vm.installPack("sqlite")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(1, gateway.calls.count { it == "install:sqlite" })
        assertEquals(PackState.Ready("1.0.0"), vm.rowState("sqlite"))
    }

    @Test
    fun a_pack_that_is_downloading_cannot_start_a_second_install() {
        val gateway =
            FakeExecutorsGateway(
                packs = listOf(pack("sqlite", installed = false)),
                installProgress = listOf(5L to 40L),
            )
        val vm = viewModel(gateway)
        gateway.beforeReport = { vm.installPack("sqlite") }

        vm.installPack("sqlite")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(1, gateway.calls.count { it == "install:sqlite" })
    }

    @Test
    fun linux_setup_steps_run_in_order_from_starting_to_setting_up() {
        val gateway =
            FakeExecutorsGateway(
                linuxIsReady = false,
                setupProgress = listOf(0L to 100L, 50L to 100L, 100L to 100L, 100L to 100L),
            )
        val vm = viewModel(gateway)
        val phases = mutableListOf<LinuxPhase>()
        gateway.beforeReport = {
            val linux = vm.state.value.linux as LinuxState.SettingUp
            phases += linuxPhaseOf(linux.downloaded, linux.total)
        }

        vm.installLinux()
        dispatcher.scheduler.advanceUntilIdle()

        // Each capture sees the state left by the report before it.
        assertEquals(
            listOf(
                LinuxPhase.Starting,
                LinuxPhase.Downloading,
                LinuxPhase.Downloading,
                LinuxPhase.SettingUp,
            ),
            phases,
        )
        assertEquals(LinuxState.Ready, vm.state.value.linux)
    }

    @Test
    fun a_failed_linux_setup_is_broken_and_repair_runs_setup_again() {
        val gateway = FakeExecutorsGateway(setupFailure = ExecutorsFailure.Unknown)
        val vm = viewModel(gateway)

        vm.installLinux()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(LinuxState.Broken(ExecutorsFailure.Unknown), vm.state.value.linux)

        gateway.setupFailure = null
        vm.repairLinux()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(LinuxState.Ready, vm.state.value.linux)
        assertEquals(2, gateway.calls.count { it == "setup" })
    }

    @Test
    fun the_play_build_has_no_linux_mode_and_makes_no_linux_calls() {
        val gateway = FakeExecutorsGateway()
        val vm = viewModel(gateway, config = build.copy(linuxSupported = false))

        vm.installLinux()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(LinuxState.Unavailable(LinuxUnavailable.ThisBuild), vm.state.value.linux)
        assertFalse(gateway.calls.any { it == "linuxReady" || it == "setup" })
    }

    @Test
    fun linux_without_a_setup_manifest_and_not_ready_is_not_configured() {
        val vm =
            viewModel(
                FakeExecutorsGateway(linuxIsReady = false),
                config = build.copy(rootfsManifestUrl = null),
            )

        assertEquals(LinuxState.Unavailable(LinuxUnavailable.NotConfigured), vm.state.value.linux)
        assertFalse(vm.state.value.linuxSetupInBuild)
    }

    @Test
    fun a_core_that_says_linux_is_not_configured_shows_not_available() {
        val vm =
            viewModel(FakeExecutorsGateway(availabilityFailure = ExecutorsFailure.NotConfigured))

        assertEquals(LinuxState.Unavailable(LinuxUnavailable.NotConfigured), vm.state.value.linux)
    }

    @Test
    fun a_ready_linux_mode_shows_ready_and_offers_repair() {
        val vm = viewModel(FakeExecutorsGateway(linuxIsReady = true))

        assertEquals(LinuxState.Ready, vm.state.value.linux)
        assertTrue(vm.state.value.linuxSetupInBuild)
    }

    @Test
    fun the_network_switch_loads_the_saved_value() {
        val vm = viewModel(FakeExecutorsGateway(network = true))

        assertEquals(NetworkState.Loaded(allowed = true), vm.state.value.network)
    }

    @Test
    fun the_network_switch_writes_the_value_and_reads_it_back() {
        val gateway = FakeExecutorsGateway(network = false)
        val vm = viewModel(gateway)

        vm.setNetworkAllowed(true)
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue("setProjectNetwork:true" in gateway.calls)
        assertEquals(NetworkState.Loaded(allowed = true), vm.state.value.network)
    }

    @Test
    fun a_network_write_that_fails_keeps_the_saved_value_and_names_the_failure() {
        val gateway = FakeExecutorsGateway(network = false)
        val vm = viewModel(gateway)
        gateway.networkFailure = ExecutorsFailure.Unavailable

        vm.setNetworkAllowed(true)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(
            NetworkState.Loaded(
                allowed = false,
                saving = false,
                failure = ExecutorsFailure.Unavailable,
            ),
            vm.state.value.network,
        )
    }

    @Test
    fun a_network_read_that_fails_can_be_tried_again() {
        val gateway = FakeExecutorsGateway(networkFailure = ExecutorsFailure.Unavailable)
        val vm = viewModel(gateway)
        assertEquals(NetworkState.LoadFailed(ExecutorsFailure.Unavailable), vm.state.value.network)

        gateway.networkFailure = null
        vm.retryNetwork()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(NetworkState.Loaded(allowed = false), vm.state.value.network)
    }

    @Test
    fun without_a_project_there_is_no_network_switch_and_no_network_call() {
        val gateway = FakeExecutorsGateway()
        val vm = viewModel(gateway, projectId = null)

        assertEquals(NetworkState.NoProject, vm.state.value.network)
        assertEquals(CiState.NoProject, vm.state.value.ci)
        assertFalse(
            gateway.calls.any {
                it.startsWith("projectNetwork") ||
                    it.startsWith("ciReadiness")
            },
        )
    }

    @Test
    fun the_ci_section_shows_the_readiness_reason() {
        val vm = viewModel(FakeExecutorsGateway(readiness = CiReadiness.NotSignedIn))

        assertEquals(CiState.Status(CiReadiness.NotSignedIn), vm.state.value.ci)
    }

    @Test
    fun a_failed_ci_readiness_check_is_shown_as_failed() {
        val vm = viewModel(FakeExecutorsGateway(readinessFailure = ExecutorsFailure.Unavailable))

        assertEquals(CiState.Failed(ExecutorsFailure.Unavailable), vm.state.value.ci)
    }
}

/** A pack summary with the version the tests expect. */
internal fun pack(
    id: String,
    installed: Boolean,
): PackSummary =
    PackSummary(id = id, version = "1.0.0", installed = installed, sizeBytes = 40_000_000L)
