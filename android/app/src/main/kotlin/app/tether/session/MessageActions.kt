package app.tether.session

import android.content.ClipData
import android.os.Build
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.core.generated.MessageView
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.TetherSheet
import app.tether.design.tokens.TetherSpace
import app.tether.messages.AppMessages
import kotlinx.coroutines.launch

/** The sheet opened by a long press on a message. Copy is its only action. */
@Composable
fun MessageActionsSheet(
    onCopy: () -> Unit,
    onDismiss: () -> Unit,
) {
    TetherSheet(
        title = stringResource(R.string.states_message_sheet_title),
        onDismiss = onDismiss,
    ) {
        ListGroup(modifier = Modifier.padding(horizontal = TetherSpace.space4)) {
            ListRow(title = stringResource(R.string.states_copy), onClick = onCopy)
        }
    }
}

/**
 * The Copy action for a message. It puts the message's text on the clipboard, then shows the
 * "Copied" toast on [toasts] when the system does not show its own. The copy runs on the screen's
 * scope, so closing the sheet does not cancel it.
 */
@Composable
fun rememberCopyMessage(toasts: AppMessages?): (MessageView) -> Unit {
    val scope = rememberCoroutineScope()
    val clipboard = LocalClipboard.current
    val label = stringResource(R.string.states_copy)
    val copied = stringResource(R.string.states_copied)
    return { message ->
        scope.launch {
            clipboard.setClipEntry(ClipEntry(ClipData.newPlainText(label, plainText(message))))
            if (showsCopiedToast(Build.VERSION.SDK_INT)) toasts?.show(copied)
        }
    }
}
