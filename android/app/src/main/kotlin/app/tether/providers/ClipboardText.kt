package app.tether.providers

import androidx.compose.ui.platform.Clipboard

/** The first text item on the clipboard, or null when it holds no text. */
suspend fun Clipboard.firstText(): String? =
    getClipEntry()
        ?.clipData
        ?.takeIf { it.itemCount > 0 }
        ?.getItemAt(0)
        ?.text
        ?.toString()
        ?.takeIf { it.isNotEmpty() }
