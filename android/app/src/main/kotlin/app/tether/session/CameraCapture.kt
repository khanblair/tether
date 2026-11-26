package app.tether.session

import android.content.ActivityNotFoundException
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.FileProvider
import java.io.File

/**
 * Opens the system camera app to take one photo into the cache folder. [onCaptured] gets the file
 * when a photo was saved. [onUnavailable] runs when no camera app answers. A photo that is not
 * saved is deleted.
 */
@Composable
internal fun rememberCameraCapture(
    onCaptured: (File) -> Unit,
    onUnavailable: () -> Unit,
): () -> Unit {
    val context = LocalContext.current
    var pendingPath by rememberSaveable { mutableStateOf<String?>(null) }
    val camera =
        rememberLauncherForActivityResult(ActivityResultContracts.TakePicture()) { saved ->
            val file = pendingPath?.let(::File)
            pendingPath = null
            if (saved && file != null) onCaptured(file) else file?.delete()
        }
    return {
        val file = File(context.cacheDir, "camera/${cameraFileName(System.currentTimeMillis())}")
        file.parentFile?.mkdirs()
        val uri: Uri =
            FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
        pendingPath = file.path
        try {
            camera.launch(uri)
        } catch (_: ActivityNotFoundException) {
            pendingPath = null
            file.delete()
            onUnavailable()
        }
    }
}
