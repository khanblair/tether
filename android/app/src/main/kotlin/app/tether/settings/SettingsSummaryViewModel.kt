package app.tether.settings

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.session.SessionMode
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * The values the main Settings list shows under each row. [storageBytes] is null when the read
 * failed, so the row can say so instead of showing a made-up number.
 */
@Immutable
data class SettingsSummary(
    val profileName: String = "",
    val defaultModel: DefaultModel? = null,
    val defaultMode: SessionMode = SessionMode.ACCEPT_EDITS,
    val appLock: Boolean = false,
    val notificationsOn: Boolean = true,
    val storageBytes: Long? = null,
)

/** Reads the values for the main Settings list on [io]. */
class SettingsSummaryViewModel(
    private val store: SettingsStore,
    private val notifications: NotificationSource,
    private val storage: StorageSource,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(SettingsSummary())

    /** The summary. */
    val state: StateFlow<SettingsSummary> = mutableState.asStateFlow()

    init {
        refresh()
    }

    /** Reads every value again, for when the screen resumes. */
    fun refresh() {
        viewModelScope.launch(io) {
            val summary =
                SettingsSummary(
                    profileName = store.profileName(),
                    defaultModel = store.defaultModel(),
                    defaultMode = store.defaultMode(),
                    appLock = store.appLockEnabled(),
                    notificationsOn = notifications.enabled(),
                    storageBytes = runCatching { storage.usage().appData }.getOrNull(),
                )
            mutableState.update { summary }
        }
    }
}
