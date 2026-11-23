package app.tether.git

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class DiffViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeGitGateway()

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
        gateway.diffText =
            "diff --git a/a.txt b/a.txt\n" +
            "--- a/a.txt\n" +
            "+++ b/a.txt\n" +
            "@@ -1 +1 @@\n" +
            "-old\n" +
            "+new\n"
    }

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun createViewModel(
        path: String? = null,
        staged: Boolean = false,
    ): DiffViewModel {
        val vm = DiffViewModel(PROJECT, path, staged, gateway, io = dispatcher)
        dispatcher.scheduler.runCurrent()
        return vm
    }

    @Test
    fun all_changes_on_the_not_staged_side_is_the_default_request() {
        val vm = createViewModel()

        assertEquals(listOf(DiffRequest(staged = false, path = null)), gateway.diffRequests)
        val content = (vm.state.value.content as Loadable.Ready<DiffContent>).value
        assertEquals("a.txt", content.files.single().path)
        assertFalse(content.truncated)
    }

    @Test
    fun one_file_passes_its_path_in_the_request() {
        createViewModel(path = "a.txt", staged = true)

        assertEquals(listOf(DiffRequest(staged = true, path = "a.txt")), gateway.diffRequests)
    }

    @Test
    fun switching_the_side_reads_the_diff_again_for_that_side() {
        val vm = createViewModel(path = "a.txt")

        vm.setStaged(true)
        dispatcher.scheduler.runCurrent()

        assertEquals(
            listOf(
                DiffRequest(staged = false, path = "a.txt"),
                DiffRequest(staged = true, path = "a.txt"),
            ),
            gateway.diffRequests,
        )
        assertTrue(vm.state.value.staged)
    }

    @Test
    fun choosing_the_side_already_shown_reads_nothing() {
        val vm = createViewModel()

        vm.setStaged(false)
        dispatcher.scheduler.runCurrent()

        assertEquals(1, gateway.diffRequests.size)
    }

    @Test
    fun a_cut_diff_is_marked_as_truncated() {
        gateway.diffText = gateway.diffText + "[diff truncated at 64 KiB]\n"

        val vm = createViewModel()

        assertTrue((vm.state.value.content as Loadable.Ready<DiffContent>).value.truncated)
    }

    @Test
    fun a_failure_then_try_again_reads_the_diff_again() {
        gateway.diffFailure = GitFailure.Unavailable
        val vm = createViewModel()
        assertEquals(Loadable.Failed(GitFailure.Unavailable), vm.state.value.content)

        gateway.diffFailure = null
        vm.retry()
        dispatcher.scheduler.runCurrent()

        assertTrue(vm.state.value.content is Loadable.Ready)
        assertEquals(2, gateway.diffRequests.size)
    }

    @Test
    fun wrapping_starts_on_and_toggles_without_a_read() {
        val vm = createViewModel()

        assertTrue(vm.state.value.wrap)
        vm.toggleWrap()

        assertFalse(vm.state.value.wrap)
        assertEquals(1, gateway.diffRequests.size)
    }

    private companion object {
        const val PROJECT = "project-1"
    }
}
