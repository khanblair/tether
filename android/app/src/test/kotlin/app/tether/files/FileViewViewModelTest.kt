package app.tether.files

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class FileViewViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeFilesGateway()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(path: String): FileViewViewModel {
        val vm = FileViewViewModel("blog", path, gateway, io = dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        return vm
    }

    @Test
    fun a_text_file_shows_its_lines() {
        gateway.previews["src/auth.kt"] =
            FilePreview(
                "src/auth.kt",
                9L,
                "fun a()\r\nfun b()\r\n",
                truncated = false,
                binary = false,
            )
        val state = viewModel("src/auth.kt").state.value
        assertFalse(state.loading)
        assertNull(state.problem)
        assertEquals(listOf("fun a()", "fun b()"), state.lines)
        assertFalse(state.preview!!.truncated)
    }

    @Test
    fun a_truncated_file_keeps_the_flag_and_the_lines_it_has() {
        gateway.previews["big.log"] =
            FilePreview("big.log", 300_000L, "line\n", truncated = true, binary = false)
        val state = viewModel("big.log").state.value
        assertTrue(state.preview!!.truncated)
        assertEquals(listOf("line"), state.lines)
    }

    @Test
    fun a_binary_file_has_no_lines_and_says_it_is_binary() {
        gateway.previews["logo.png"] =
            FilePreview("logo.png", 4_096L, null, truncated = false, binary = true)
        val state = viewModel("logo.png").state.value
        assertTrue(state.preview!!.binary)
        assertEquals(emptyList<String>(), state.lines)
        assertNull(state.problem)
    }

    @Test
    fun a_failed_read_sets_the_problem_and_retry_reads_again() {
        gateway.readFailure = FilesFailure(FilesProblem.NotFound)
        val vm = viewModel("gone.kt")
        assertEquals(FilesProblem.NotFound, vm.state.value.problem)
        assertNull(vm.state.value.preview)

        gateway.readFailure = null
        gateway.previews["gone.kt"] =
            FilePreview("gone.kt", 2L, "x", truncated = false, binary = false)
        vm.retry()
        dispatcher.scheduler.advanceUntilIdle()
        assertNull(vm.state.value.problem)
        assertEquals(listOf("x"), vm.state.value.lines)
        assertEquals(listOf("gone.kt", "gone.kt"), gateway.readCalls)
    }
}
