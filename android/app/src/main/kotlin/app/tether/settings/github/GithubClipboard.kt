package app.tether.settings.github

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context

/** Puts [text] on the clipboard under a plain label. Does nothing when no clipboard is there. */
internal fun copyToClipboard(
    context: Context,
    text: String,
) {
    val manager = context.getSystemService(ClipboardManager::class.java) ?: return
    manager.setPrimaryClip(ClipData.newPlainText("", text))
}

/** The first text item on the clipboard, trimmed, or null when there is none. */
internal fun readClipboardText(context: Context): String? {
    val manager = context.getSystemService(ClipboardManager::class.java)
    val item = manager?.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)
    return item
        ?.coerceToText(context)
        ?.toString()
        ?.trim()
        ?.takeIf { it.isNotEmpty() }
}
