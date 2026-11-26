package app.tether.preview

import app.tether.files.FilesProblem
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class PreviewViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakePreviewFilesGateway()
    private var seeded = false

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(
        path: String = "index.html",
        network: Boolean = false,
    ): PreviewViewModel {
        seeded = network
        val target = PreviewTarget.File("blog", path)
        val vm = PreviewViewModel(target, gateway, { seeded }, io = dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        return vm
    }

    @Test
    fun an_existing_file_loads_its_page_with_the_network_off_by_default() {
        gateway.files["index.html"] = "<h1>hi</h1>".toByteArray()
        val vm = viewModel()
        assertEquals(PreviewLoad.Page("$PREVIEW_ORIGIN/index.html"), vm.state.value.load)
        assertFalse(vm.state.value.networkAllowed)
    }

    @Test
    fun the_probe_reads_one_byte_and_does_not_load_the_file() {
        gateway.files["index.html"] = ByteArray(5_000)
        viewModel()
        assertEquals(listOf("index.html:1"), gateway.reads)
    }

    @Test
    fun a_missing_file_shows_the_missing_state() {
        val vm = viewModel()
        assertEquals(PreviewLoad.Missing, vm.state.value.load)
    }

    @Test
    fun a_failed_read_shows_the_failure_state_and_retry_loads_it() {
        gateway.files["index.html"] = byteArrayOf(1)
        gateway.failing += "index.html"
        val vm = viewModel()
        assertEquals(PreviewLoad.Failed(FilesProblem.Unavailable), vm.state.value.load)

        gateway.failing.clear()
        vm.reload()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(PreviewLoad.Page("$PREVIEW_ORIGIN/index.html"), vm.state.value.load)
    }

    @Test
    fun the_network_setting_is_seeded_from_the_project() {
        gateway.files["index.html"] = byteArrayOf(1)
        assertTrue(viewModel(network = true).state.value.networkAllowed)
    }

    @Test
    fun a_failed_seed_counts_as_network_off() {
        gateway.files["index.html"] = byteArrayOf(1)
        val target = PreviewTarget.File("blog", "index.html")
        val vm =
            PreviewViewModel(
                target,
                gateway,
                { throw IllegalStateException("no core") },
                dispatcher,
            )
        dispatcher.scheduler.advanceUntilIdle()
        assertFalse(vm.state.value.networkAllowed)
        assertEquals(PreviewLoad.Page("$PREVIEW_ORIGIN/index.html"), vm.state.value.load)
    }

    @Test
    fun each_distinct_blocked_url_counts_once() {
        gateway.files["index.html"] = byteArrayOf(1)
        val vm = viewModel()
        vm.onBlocked("https://example.com/a.js")
        vm.onBlocked("https://example.com/a.js")
        vm.onBlocked("https://example.com/b.css")
        assertEquals(2, vm.state.value.blocked.size)
    }

    @Test
    fun turning_the_network_on_lets_requests_through_and_reloads_the_page() {
        gateway.files["index.html"] = byteArrayOf(1)
        val vm = viewModel()
        vm.onBlocked("https://example.com/a.js")
        val before = vm.state.value.reloadKey

        vm.setNetworkAllowed(true)

        assertTrue(vm.networkAllowed())
        assertTrue(
            vm.state.value.blocked
                .isEmpty(),
        )
        assertEquals(before + 1, vm.state.value.reloadKey)
    }

    @Test
    fun a_reload_starts_the_blocked_count_over() {
        gateway.files["index.html"] = byteArrayOf(1)
        val vm = viewModel()
        vm.onBlocked("https://example.com/a.js")
        vm.reload()
        assertTrue(
            vm.state.value.blocked
                .isEmpty(),
        )
    }

    @Test
    fun a_turn_on_before_the_seed_lands_wins_over_the_seed() {
        gateway.files["index.html"] = byteArrayOf(1)
        seeded = true
        val target = PreviewTarget.File("blog", "index.html")
        val vm = PreviewViewModel(target, gateway, { seeded }, dispatcher)
        vm.setNetworkAllowed(false)
        dispatcher.scheduler.advanceUntilIdle()
        assertFalse(vm.state.value.networkAllowed)
    }

    @Test
    fun readFile_serves_any_file_of_the_project_up_to_the_limit() {
        gateway.files["css/site.css"] = "body{}".toByteArray()
        val vm = viewModel()
        val bytes = vm.readFile("css/site.css")
        assertEquals(6L, bytes.sizeBytes)
        assertFalse(bytes.truncated)
        assertEquals("css/site.css:10485760", gateway.reads.last())
    }

    @Test
    fun an_inline_preview_loads_its_html_and_has_no_files() {
        val target = PreviewTarget.Inline("<p>x</p>")
        val vm = PreviewViewModel(target, gateway, { false }, dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(PreviewLoad.Html("<p>x</p>"), vm.state.value.load)
        assertTrue(runCatching { vm.readFile("index.html") }.isFailure)
    }

    @Test
    fun an_inline_preview_loads_its_html_with_the_network_off_and_locked() {
        val vm = PreviewViewModel(PreviewTarget.Inline("<p>hi</p>"), gateway, { true }, dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        vm.setNetworkAllowed(true)
        assertEquals(PreviewLoad.Html("<p>hi</p>"), vm.state.value.load)
        assertFalse(vm.state.value.networkAllowed)
        assertTrue(vm.state.value.networkLocked)
    }
}
