package app.tether.preview

import app.tether.files.FilesFailure
import app.tether.files.FilesProblem
import app.tether.preview.PreviewLoad.Page
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ServerPreviewStateTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun a_server_preview_shows_the_page_until_its_process_stops() {
        val vm =
            PreviewViewModel(
                PreviewTarget.Server("http://127.0.0.1:5173/", "one"),
                FakePreviewFilesGateway(),
                { false },
                dispatcher,
            )
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(Page("http://127.0.0.1:5173/"), vm.state.value.load)
        assertEquals(ServerOrigin(5173), vm.state.value.serverOrigin)

        vm.setServerRunning(false)

        assertFalse(vm.state.value.serverRunning)
        val failure = runCatching { vm.readFile("index.html") }.exceptionOrNull()
        assertEquals(FilesProblem.NotFound, (failure as FilesFailure).problem)
    }
}
