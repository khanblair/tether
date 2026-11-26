package app.tether.session

import android.content.Context
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.messages.AppMessages
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private const val FALLBACK_NAME = "tether-session"
private const val MAX_NAME_LENGTH = 60
private const val MARKDOWN_TYPE = "text/markdown"

/**
 * A Markdown file name from [title]: its words joined by dashes, with only letters, digits and
 * dashes kept, and ".md" added. A title with nothing left gives "tether-session.md".
 */
internal fun exportFileName(title: String): String {
    val base =
        title
            .trim()
            .replace(Regex("\\s+"), "-")
            .filter { it.isLetterOrDigit() || it == '-' }
            .take(MAX_NAME_LENGTH)
            .trim('-')
    return "${base.ifEmpty { FALLBACK_NAME }}.md"
}

/**
 * Returns a function that opens the system "save as" picker for the session's Markdown export.
 * The picker suggests the session's name. Call it where the composable stays on screen while a
 * sheet closes, because the picker's result needs its launcher to stay registered. [messages]
 * shows whether the save worked.
 */
@Composable
fun rememberExportMarkdown(
    connection: SessionConnection,
    messages: AppMessages?,
): () -> Unit {
    val context = LocalContext.current.applicationContext
    val scope = rememberCoroutineScope()
    val exported = stringResource(R.string.compose_command_export_done)
    val failed = stringResource(R.string.compose_command_failed)
    val picker =
        rememberLauncherForActivityResult(
            ActivityResultContracts.CreateDocument(MARKDOWN_TYPE),
        ) { uri ->
            uri?.let { target ->
                scope.launch {
                    val saved = writeExport(context, connection, target)
                    messages?.show(if (saved) exported else failed)
                }
            }
        }
    return {
        scope.launch {
            val name = suggestedName(connection)
            picker.launch(name)
        }
    }
}

/** The file name for the session's title. A title that cannot be read gives the fallback name. */
private suspend fun suggestedName(connection: SessionConnection): String =
    sessionCall(Dispatchers.IO) { connection.settings.details(connection.sessionId).title }
        .map(::exportFileName)
        .getOrElse { exportFileName("") }

/** Writes the session as Markdown to [uri]. It is false when the write failed. */
@Suppress("TooGenericExceptionCaught")
private suspend fun writeExport(
    context: Context,
    connection: SessionConnection,
    uri: Uri,
): Boolean =
    withContext(Dispatchers.IO) {
        try {
            val markdown = connection.exportMarkdown().toByteArray()
            context.contentResolver
                .openOutputStream(uri, "w")
                ?.use { it.write(markdown) } != null
        } catch (_: Exception) {
            false
        }
    }
