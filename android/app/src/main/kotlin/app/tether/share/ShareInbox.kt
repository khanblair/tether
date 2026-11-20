package app.tether.share

import android.content.Intent
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** One pending share from Android's text share sheet. */
data class SharedText(val value: String, val projectId: String? = null)

/** Keeps the share until the user picks a project or dismisses it. */
class ShareInbox {
    private val pendingValue = MutableStateFlow<SharedText?>(null)
    val pending: StateFlow<SharedText?> = pendingValue.asStateFlow()

    fun accept(intent: Intent?) {
        if (intent?.action != Intent.ACTION_SEND || intent.type != "text/plain") return
        acceptText(intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString())
    }

    internal fun acceptText(value: String?) {
        val text = value?.trim().orEmpty()
        if (text.isNotEmpty()) pendingValue.value = SharedText(text.take(MAX_SHARED_TEXT))
    }

    fun chooseProject(id: String) {
        pendingValue.value = pendingValue.value?.copy(projectId = id)
    }

    fun clear() {
        pendingValue.value = null
    }

    companion object {
        private const val MAX_SHARED_TEXT = 32_768
    }
}
