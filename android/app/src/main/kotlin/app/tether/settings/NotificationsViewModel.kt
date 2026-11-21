package app.tether.settings

import android.app.NotificationManager
import androidx.annotation.StringRes
import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/** One channel row. [on] is false when the user turned the channel off in system settings. */
@Immutable
data class ChannelRow(
    val id: String,
    @StringRes val nameRes: Int,
    @StringRes val descriptionRes: Int,
    val on: Boolean,
)

/** What the notifications screen shows. [enabled] false is the "off in system settings" state. */
@Immutable
data class NotificationsUiState(
    val enabled: Boolean = true,
    val canAskForPermission: Boolean = false,
    val channels: List<ChannelRow> = emptyList(),
)

/**
 * Reads the system state. [refresh] runs when the screen resumes, since the user may have changed
 * it.
 */
class NotificationsViewModel(
    private val source: NotificationSource,
    private val specs: List<ChannelSpec> = TETHER_CHANNELS,
) : ViewModel() {
    private val mutableState = MutableStateFlow(read())

    /** The screen state. */
    val state: StateFlow<NotificationsUiState> = mutableState.asStateFlow()

    /** Reads the system state again. */
    fun refresh() {
        mutableState.update { read() }
    }

    private fun read(): NotificationsUiState =
        NotificationsUiState(
            enabled = source.enabled(),
            canAskForPermission = source.canAskForPermission(),
            channels =
                specs.map { spec ->
                    ChannelRow(
                        id = spec.id,
                        nameRes = spec.nameRes,
                        descriptionRes = spec.descriptionRes,
                        on =
                            (source.importance(spec.id) ?: NotificationManager.IMPORTANCE_NONE) !=
                                NotificationManager.IMPORTANCE_NONE,
                    )
                },
        )
}
