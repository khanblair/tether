package app.tether.linux

import app.tether.core.generated.ExecutorRuntimeException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class LinuxShellViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun openAndOutputReachTheFrame() =
        runTest {
            val gateway = FakeLinuxShellGateway()
            val model = LinuxShellViewModel(gateway, dispatcher)
            model.open("project", 4, 20)
            advanceUntilIdle()
            assertEquals(LinuxShellState.Open, model.state.value)
            gateway.emit("hello")
            advanceUntilIdle()
            assertTrue(
                model.frame.value.lines.any { line ->
                    line.joinToString("") { it.character }.contains("hello")
                },
            )
            model.clear()
            assertTrue(
                model.frame.value.lines
                    .last()
                    .all { it.character == " " },
            )
        }

    @Test
    fun exitIsReflectedInState() =
        runTest {
            val gateway = FakeLinuxShellGateway()
            val model = LinuxShellViewModel(gateway, dispatcher)
            model.open(null, 4, 20)
            advanceUntilIdle()
            gateway.exit(7)
            advanceUntilIdle()
            assertEquals(LinuxShellState.Exited(7), model.state.value)
        }

    @Test
    fun mapsNotReadyAndAlreadyOpen() =
        runTest {
            val notReady = FakeLinuxShellGateway(ExecutorRuntimeException.LinuxNotReady())
            val notReadyModel = LinuxShellViewModel(notReady, dispatcher)
            notReadyModel.open(null, 4, 20)
            advanceUntilIdle()
            assertEquals(LinuxShellState.NotReady, notReadyModel.state.value)

            val busy = FakeLinuxShellGateway(ExecutorRuntimeException.LinuxShellAlreadyOpen())
            val busyModel = LinuxShellViewModel(busy, dispatcher)
            busyModel.open(null, 4, 20)
            advanceUntilIdle()
            assertEquals(LinuxShellState.Busy, busyModel.state.value)
        }

    @Test
    fun oneViewModelKeepsTheBufferAcrossScreenRecreation() =
        runTest {
            val gateway = FakeLinuxShellGateway()
            val model = LinuxShellViewModel(gateway, dispatcher)
            model.open(null, 4, 20)
            advanceUntilIdle()
            gateway.emit("persisted")
            advanceUntilIdle()
            assertTrue(
                model.frame.value.lines
                    .flatten()
                    .any { it.character == "p" },
            )
            model.closeShell()
            advanceUntilIdle()
            assertEquals(LinuxShellState.Closed, model.state.value)
        }
}
