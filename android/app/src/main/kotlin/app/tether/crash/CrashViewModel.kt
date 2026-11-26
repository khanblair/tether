package app.tether.crash

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** The last crash report for the About screen. Clearing it removes the card. */
class CrashViewModel(
    private val store: CrashStore,
) : ViewModel() {
    private val reportState = MutableStateFlow(store.read())

    /** The saved report, or null when there is none to show. */
    val report: StateFlow<String?> = reportState.asStateFlow()

    /** Removes the saved report and hides the card. */
    fun clear() {
        store.clear()
        reportState.value = null
    }

    companion object {
        fun factory(store: CrashStore) =
            viewModelFactory {
                initializer { CrashViewModel(store) }
            }
    }
}
