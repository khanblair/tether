package app.tether.files

import android.content.ClipData
import android.os.Build
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.messages.AppMessages
import app.tether.session.showsCopiedToast
import kotlinx.coroutines.launch

/**
 * The Copy path action. It puts the path on the clipboard, then shows the "Path copied" toast on
 * [toasts] only where the system shows no confirmation of its own, as the message copy does.
 */
@Composable
fun rememberCopyPath(toasts: AppMessages): (String) -> Unit {
    val scope = rememberCoroutineScope()
    val clipboard = LocalClipboard.current
    val label = stringResource(R.string.files_copy_path)
    val copied = stringResource(R.string.files_path_copied)
    return { path ->
        scope.launch {
            clipboard.setClipEntry(ClipEntry(ClipData.newPlainText(label, path)))
            if (showsCopiedToast(Build.VERSION.SDK_INT)) toasts.show(copied)
        }
    }
}
