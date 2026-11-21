package app.tether.session

import android.os.Build
import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView

/** The text parts of [message], joined by a blank line. Empty when the message has no text. */
internal fun plainText(message: MessageView): String =
    message.parts
        .filterIsInstance<MessagePartView.Text>()
        .joinToString(separator = "\n\n") { it.text }

/** The text of the newest user message, which Retry sends again. Null when none has text. */
internal fun lastUserText(messages: List<MessageView>): String? =
    messages
        .lastOrNull { it.role == RoleView.USER }
        ?.let(::plainText)
        ?.takeIf(String::isNotBlank)

/**
 * Whether a copy shows Tether's own "Copied" toast. From Android 13 the system shows its own
 * clipboard confirmation, so Tether stays quiet there.
 */
internal fun showsCopiedToast(sdkInt: Int): Boolean = sdkInt < Build.VERSION_CODES.TIRAMISU
