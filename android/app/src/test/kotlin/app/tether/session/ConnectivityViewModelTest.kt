package app.tether.session

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ConnectivityViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private class FakeConnectivity(
        online: Boolean,
    ) : ConnectivitySource {
        val state = MutableStateFlow(online)
        override val online: Flow<Boolean> = state
    }

    @Test
    fun noBannerWhileTheNetworkIsUp() {
        val source = FakeConnectivity(online = true)
        val model = ConnectivityViewModel(source)

        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(model.offline.value)
    }

    @Test
    fun bannerAppearsWhenTheNetworkDrops() {
        val source = FakeConnectivity(online = true)
        val model = ConnectivityViewModel(source)

        source.state.value = false
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(model.offline.value)
    }

    @Test
    fun bannerDisappearsWhenTheNetworkReturns() {
        val source = FakeConnectivity(online = false)
        val model = ConnectivityViewModel(source)
        dispatcher.scheduler.advanceUntilIdle()

        source.state.value = true
        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(model.offline.value)
    }

    @Test
    fun aSourceThatIsOfflineFromTheStartShowsTheBanner() {
        val model = ConnectivityViewModel(FakeConnectivity(online = false))

        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(model.offline.value)
    }
}
