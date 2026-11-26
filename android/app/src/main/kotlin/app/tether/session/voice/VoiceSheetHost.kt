package app.tether.session.voice

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.core.net.toUri
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle

/**
 * Holds the voice sheet. The returned function opens it. When the user taps Insert text, the
 * transcript goes to [onInsert]. The sheet asks for the microphone on open, and it creates the
 * recognizer only while open, so the recognizer is released when the sheet closes.
 */
@Composable
internal fun rememberVoiceSheet(onInsert: (String) -> Unit): () -> Unit {
    var open by remember { mutableStateOf(false) }
    if (open) {
        VoiceSheetHost(onInsert = onInsert, onClose = { open = false })
    }
    return { open = true }
}

@Composable
private fun VoiceSheetHost(
    onInsert: (String) -> Unit,
    onClose: () -> Unit,
) {
    val context = LocalContext.current
    val recognizer = remember { AndroidVoiceRecognizer(context) }
    DisposableEffect(recognizer) {
        onDispose { recognizer.release() }
    }
    val model = remember(recognizer) { VoiceSheetViewModel(recognizer) }
    val state by model.state.collectAsStateWithLifecycle()
    val permission =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            val activity = context.findActivity()
            val canAsk = activity?.shouldShowRequestPermissionRationale(MIC) == true
            model.onPermission(granted = granted, canAskAgain = canAsk)
        }
    LaunchedEffect(Unit) { permission.launch(MIC) }
    LifecycleEventEffect(Lifecycle.Event.ON_STOP) { model.stop() }
    ListeningSheet(
        state = state,
        onAllow = {
            val denied = state as? VoiceSheetState.Denied
            if (denied?.canAskAgain == true) permission.launch(MIC) else openAppSettings(context)
        },
        onTryAgain = model::retry,
        onInsert = {
            model.insert()?.let(onInsert)
            onClose()
        },
        onCancel = {
            model.cancel()
            onClose()
        },
    )
}

private const val MIC = Manifest.permission.RECORD_AUDIO

private fun openAppSettings(context: Context) {
    context.startActivity(
        Intent(
            Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
            "package:${context.packageName}".toUri(),
        ),
    )
}

private tailrec fun Context.findActivity(): Activity? =
    when (this) {
        is Activity -> this
        is ContextWrapper -> baseContext.findActivity()
        else -> null
    }
