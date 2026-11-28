package app.tether.linux

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/**
 * One key mapper shared by the key row and the Android keyboard, so an armed Ctrl or Alt applies
 * to whichever of them sends the next key.
 */
@Stable
internal class TerminalKeyboard(
    private val onInput: (ByteArray) -> Unit,
) {
    private val keys = TerminalKeys()

    /** Whether Ctrl is armed for the next key. */
    var ctrlArmed by mutableStateOf(false)
        private set

    /** Whether Alt is armed for the next key. */
    var altArmed by mutableStateOf(false)
        private set

    /** Handles a key row button. */
    fun press(key: TerminalKey) = send(keys.press(key))

    /** Handles text from the Android keyboard. */
    fun type(text: String) = send(keys.type(text))

    private fun send(bytes: ByteArray) {
        if (bytes.isNotEmpty()) onInput(bytes)
        ctrlArmed = keys.ctrlArmed
        altArmed = keys.altArmed
    }
}
