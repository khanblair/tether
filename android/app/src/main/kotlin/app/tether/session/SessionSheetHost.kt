package app.tether.session

import androidx.activity.compose.LocalActivity
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import app.tether.MainActivity
import app.tether.R
import app.tether.messages.AppMessages
import app.tether.providers.ProviderGateway

/** The sheets a session screen can open. */
enum class SessionSheetKind { Model, Mode, Menu, Rename, Rewind }

/** One open sheet. [token] is new for each opening, so its view model starts fresh. */
data class OpenSheet(
    val kind: SessionSheetKind,
    val token: Long = System.nanoTime(),
)

/**
 * The open sheet and the reload count of one session screen. A sheet that saves a change calls
 * [applied], which closes it and makes the screen reload its transcript.
 */
class SessionSheetControl {
    /** The sheet on screen, or null. */
    var current: OpenSheet? by mutableStateOf(null)
        private set

    /** Counts saved changes, so the screen knows to reload. */
    var reloads: Int by mutableIntStateOf(0)
        private set

    /** Opens [kind] as a new sheet. */
    fun openSheet(kind: SessionSheetKind) {
        current = OpenSheet(kind)
    }

    /** A change was saved: close the sheet and ask the screen to reload. */
    fun applied() {
        current = null
        reloads++
    }

    /** Closes the sheet without a change. */
    fun dismiss() {
        current = null
    }
}

/**
 * What the session sheets need from the app: the saved keys, the snackbar host, and the way to
 * the Providers screen. The session screen builds it from the app module.
 */
class SessionSheetEnv(
    val providers: ProviderGateway,
    val messages: AppMessages,
    val openProviders: () -> Unit,
)

/**
 * The sheet slot of a session screen. It turns the screen's state into [SessionSheetDeps], and
 * asks the activity for the biometric check that Bypass needs. [isRunning] is read each time, so a
 * sheet sees a turn that starts while it is open.
 */
@Composable
fun SessionSheetsSlot(
    connection: SessionConnection?,
    sheets: SessionSheetEnv?,
    control: SessionSheetControl,
    isRunning: () -> Boolean,
) {
    val activity = LocalActivity.current as? MainActivity
    val bypassTitle = stringResource(R.string.session_bypass_auth_title)
    val open = control.current
    if (open == null || sheets == null || connection == null) return
    SessionSheetHost(
        open = open,
        deps =
            SessionSheetDeps(
                sessionId = connection.sessionId,
                settings = connection.settings,
                providers = sheets.providers,
                rewinder = connection.rewinder,
                isRunning = isRunning,
                reopen = connection::reopen,
            ),
        env = sheets,
        running = isRunning(),
        authenticate = { onSuccess, onError ->
            if (activity == null) {
                onError()
            } else {
                activity.authenticateForKeyReveal(bypassTitle, onSuccess, onError)
            }
        },
        onOpen = control::openSheet,
        onApplied = control::applied,
        onDismiss = control::dismiss,
    )
}

/**
 * Shows the open sheet. Every sheet reads its state from a view model keyed by [OpenSheet.token].
 * [onApplied] runs after a change is saved, so the screen reloads. [onDismiss] closes the sheet.
 */
@Composable
fun SessionSheetHost(
    open: OpenSheet,
    deps: SessionSheetDeps,
    env: SessionSheetEnv,
    running: Boolean,
    authenticate: (onSuccess: () -> Unit, onError: () -> Unit) -> Unit,
    onOpen: (SessionSheetKind) -> Unit,
    onApplied: () -> Unit,
    onDismiss: () -> Unit,
) {
    when (open.kind) {
        SessionSheetKind.Model ->
            ModelSheetHost(open, deps, env, running, onApplied, onDismiss)
        SessionSheetKind.Mode ->
            ModeSheetHost(open, deps, running, authenticate, onApplied, onDismiss)
        SessionSheetKind.Menu ->
            SessionMenuSheet(
                onRename = { onOpen(SessionSheetKind.Rename) },
                onRewind = { onOpen(SessionSheetKind.Rewind) },
                onDismiss = onDismiss,
            )
        SessionSheetKind.Rename -> RenameSheetHost(open, deps, onApplied, onDismiss)
        SessionSheetKind.Rewind ->
            RewindSheetHost(open, deps, env, onApplied, onDismiss)
    }
}

@Composable
private fun ModelSheetHost(
    open: OpenSheet,
    deps: SessionSheetDeps,
    env: SessionSheetEnv,
    running: Boolean,
    onApplied: () -> Unit,
    onDismiss: () -> Unit,
) {
    val viewModel: ModelSheetViewModel =
        viewModel(
            key = "model-${open.token}",
            factory = ModelSheetViewModel.factory(deps, onApplied),
        )
    val state by viewModel.state.collectAsStateWithLifecycle()
    ModelSheet(
        state = state,
        running = running,
        onChoose = viewModel::choose,
        onOpenProviders = {
            onDismiss()
            env.openProviders()
        },
        onDismiss = onDismiss,
    )
}

@Composable
private fun ModeSheetHost(
    open: OpenSheet,
    deps: SessionSheetDeps,
    running: Boolean,
    authenticate: (onSuccess: () -> Unit, onError: () -> Unit) -> Unit,
    onApplied: () -> Unit,
    onDismiss: () -> Unit,
) {
    val viewModel: ModeSheetViewModel =
        viewModel(
            key = "mode-${open.token}",
            factory = ModeSheetViewModel.factory(deps, authenticate, onApplied),
        )
    val state by viewModel.state.collectAsStateWithLifecycle()
    ModeSheet(
        state = state,
        running = running,
        onChoose = viewModel::choose,
        onConfirmBypass = viewModel::confirmBypass,
        onCancelBypass = viewModel::cancelBypass,
        onDismiss = onDismiss,
    )
}

@Composable
private fun RenameSheetHost(
    open: OpenSheet,
    deps: SessionSheetDeps,
    onApplied: () -> Unit,
    onDismiss: () -> Unit,
) {
    val viewModel: RenameViewModel =
        viewModel(key = "rename-${open.token}", factory = RenameViewModel.factory(deps, onApplied))
    val state by viewModel.state.collectAsStateWithLifecycle()
    RenameSheet(
        state = state,
        onTitleChange = viewModel::onTitleChange,
        onSave = viewModel::save,
        onDismiss = onDismiss,
    )
}

@Composable
private fun RewindSheetHost(
    open: OpenSheet,
    deps: SessionSheetDeps,
    env: SessionSheetEnv,
    onApplied: () -> Unit,
    onDismiss: () -> Unit,
) {
    val rewound = stringResource(R.string.session_rewound_toast)
    val now = remember { System.currentTimeMillis() }
    val viewModel: RewindViewModel =
        viewModel(
            key = "rewind-${open.token}",
            factory =
                RewindViewModel.factory(
                    deps = deps,
                    onApplied = onApplied,
                    onRewound = { env.messages.show(rewound) },
                ),
        )
    val state by viewModel.state.collectAsStateWithLifecycle()
    RewindSheet(
        state = state,
        now = now,
        onSelect = viewModel::select,
        onScope = viewModel::setScope,
        onRewind = viewModel::requestRewind,
        onConfirm = viewModel::confirmRewind,
        onDismissConfirm = viewModel::dismissConfirm,
        onDismiss = onDismiss,
    )
}
