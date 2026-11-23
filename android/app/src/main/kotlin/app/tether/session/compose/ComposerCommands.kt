package app.tether.session.compose

import android.content.Context
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.platform.LocalContext
import app.tether.R
import app.tether.messages.AppMessages
import app.tether.session.SessionConnection
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The composer's send action with slash commands. Plain text goes to [sendPlain] at once. Text
 * that starts with a command name is resolved off the main thread, then run, or answered with a
 * notice in the snackbar. Without [hooks] every text is plain, so a session screen with no
 * sheets sends as before.
 */
@Composable
internal fun rememberCommandSend(
    connection: SessionConnection,
    hooks: CommandHooks?,
    messages: AppMessages?,
    sendPlain: (String) -> Unit,
): (String) -> Unit {
    val context = LocalContext.current.applicationContext
    val scope = rememberCoroutineScope()
    val words = rememberCommandWords()
    val runWords = remember(context) { runWordsFrom(context) }
    val dispatcher = remember(connection) { CommandDispatcher(connection.commands) }
    val send by rememberUpdatedState(sendPlain)
    val exporter =
        rememberLauncherForActivityResult(
            ActivityResultContracts.CreateDocument("text/markdown"),
        ) { uri ->
            uri?.let { scope.saveExport(context, connection, it, messages, runWords) }
        }
    val ui =
        object : CommandUi {
            override fun notify(text: String) {
                messages?.show(text)
            }

            override fun sendPrompt(text: String) = send(text)

            override fun chooseExportFile() = exporter.launch(EXPORT_FILE_NAME)
        }
    if (hooks == null) return sendPlain
    val runner = CommandRunner(connection, hooks, ui, runWords)
    return { text ->
        if (isCommandText(text)) {
            scope.launch {
                when (
                    val decision =
                        withContext(
                            Dispatchers.IO,
                        ) { dispatcher.decide(text, words) }
                ) {
                    is SendDecision.SendText -> send(decision.text)
                    is SendDecision.Run -> runner.run(decision.effect)
                    is SendDecision.Notice -> messages?.show(decision.text)
                }
            }
        } else {
            send(text)
        }
    }
}

private const val EXPORT_FILE_NAME = "tether-session.md"

/** Writes the session as Markdown to [uri] and says whether it worked. */
@Suppress("TooGenericExceptionCaught")
private fun CoroutineScope.saveExport(
    context: Context,
    connection: SessionConnection,
    uri: Uri,
    messages: AppMessages?,
    words: RunWords,
) {
    launch {
        val saved =
            withContext(Dispatchers.IO) {
                try {
                    val markdown = connection.exportMarkdown().toByteArray()
                    context.contentResolver
                        .openOutputStream(
                            uri,
                            "w",
                        )?.use { it.write(markdown) } !=
                        null
                } catch (_: Exception) {
                    false
                }
            }
        messages?.show(if (saved) words.exported else words.failed)
    }
}

private fun runWordsFrom(context: Context): RunWords =
    RunWords(
        cleared = context.getString(R.string.compose_command_cleared),
        planOn = context.getString(R.string.compose_command_plan_on),
        exported = context.getString(R.string.compose_command_export_done),
        failed = context.getString(R.string.compose_command_failed),
        notAvailable = context.getString(R.string.compose_command_not_yet),
    )
