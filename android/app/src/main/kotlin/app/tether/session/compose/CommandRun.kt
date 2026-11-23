package app.tether.session.compose

import app.tether.session.SessionConnection
import app.tether.session.SessionMode
import app.tether.session.SessionSheetControl
import app.tether.session.SessionSheetEnv
import app.tether.session.SessionSheetKind
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** What the session screen lets a slash command do. Each one runs on the main thread. */
class CommandHooks(
    val openSheet: (SessionSheetKind) -> Unit,
    val openScreen: (CommandScreen, String) -> Unit,
    val newSession: () -> Unit,
    val applied: () -> Unit,
)

/** The hooks a slash command uses, from the session's [env] and its open sheet [control]. */
fun commandHooksOf(
    env: SessionSheetEnv,
    control: SessionSheetControl,
): CommandHooks =
    CommandHooks(
        openSheet = control::openSheet,
        openScreen = env.openScreen,
        newSession = env.newSession,
        applied = control::applied,
    )

/** What the composer lets a slash command show or send. */
interface CommandUi {
    /** Shows [text] in the snackbar. */
    fun notify(text: String)

    /** Sends [text] to the model as a turn. */
    fun sendPrompt(text: String)

    /** Asks the person where to save the Markdown export. */
    fun chooseExportFile()
}

/** The sentences a command run can show, read from the string resources by the caller. */
class RunWords(
    val cleared: String,
    val planOn: String,
    val exported: String,
    val failed: String,
    val notAvailable: String,
)

/** Runs the effect of one slash command on [connection]. Core calls run on the IO dispatcher. */
class CommandRunner(
    private val connection: SessionConnection,
    private val hooks: CommandHooks,
    private val ui: CommandUi,
    private val words: RunWords,
) {
    /** Runs [effect]. A failed core call shows [RunWords.failed] and changes nothing else. */
    suspend fun run(effect: CommandEffect) {
        when (effect) {
            is CommandEffect.SendPrompt -> ui.sendPrompt(effect.text)
            CommandEffect.NewSession -> hooks.newSession()
            CommandEffect.ClearContext -> core(words.cleared) { connection.clearContext() }
            CommandEffect.SetPlanMode -> core(words.planOn) { setPlan() }
            CommandEffect.ExportSession -> ui.chooseExportFile()
            CommandEffect.OpenModelSheet -> hooks.openSheet(SessionSheetKind.Model)
            CommandEffect.OpenModeSheet -> hooks.openSheet(SessionSheetKind.Mode)
            CommandEffect.OpenRewindSheet -> hooks.openSheet(SessionSheetKind.Rewind)
            is CommandEffect.OpenScreen -> hooks.openScreen(effect.target, effect.arguments)
            is CommandEffect.Unsupported -> ui.notify(words.notAvailable)
        }
    }

    /** Switches to Plan mode, then asks the screen to reload as the mode sheet does. */
    private fun setPlan() {
        connection.settings.setMode(connection.sessionId, SessionMode.PLAN)
        connection.reopen()
    }

    @Suppress("TooGenericExceptionCaught")
    private suspend fun core(
        done: String,
        block: () -> Unit,
    ) {
        val ok =
            withContext(Dispatchers.IO) {
                try {
                    block()
                    true
                } catch (_: Exception) {
                    false
                }
            }
        if (ok) hooks.applied()
        ui.notify(if (ok) done else words.failed)
    }
}
