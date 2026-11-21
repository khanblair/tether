package app.tether.session

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn

/**
 * Whether the session screen shows the offline banner. The app does not resume a turn when the
 * phone reconnects, so the banner says that messages fail until then.
 */
class ConnectivityViewModel(
    source: ConnectivitySource,
) : ViewModel() {
    /** True while the phone has no internet access. It stays false until the source reports. */
    val offline: StateFlow<Boolean> =
        source.online
            .map { online -> !online }
            .stateIn(viewModelScope, SharingStarted.Eagerly, initialValue = false)
}

/** Whether the phone is offline, from a [ConnectivityViewModel] scoped to the current route. */
@Composable
fun rememberOffline(): Boolean {
    val appContext = LocalContext.current.applicationContext
    val model: ConnectivityViewModel =
        viewModel(
            factory =
                viewModelFactory {
                    initializer { ConnectivityViewModel(AndroidConnectivitySource(appContext)) }
                },
        )
    return model.offline.collectAsStateWithLifecycle().value
}
