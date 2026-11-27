package app.tether.di

import android.content.Context
import android.util.Log
import app.tether.appearance.AppearanceController
import app.tether.appearance.PrefsAppearanceStore
import app.tether.core.CoreClient
import app.tether.core.generated.CiRuntime
import app.tether.core.generated.ProjectRuntime
import app.tether.core.generated.ProviderSettings
import app.tether.core.generated.SessionRuntime
import app.tether.core.generated.Updater
import app.tether.core.keystoreSecretVault
import app.tether.design.ThemeMode
import app.tether.files.FilesGateway
import app.tether.files.createFilesGateway
import app.tether.files.createPreviewFilesGateway
import app.tether.git.GitGateway
import app.tether.git.createGitGateway
import app.tether.home.CoreProjectGateway
import app.tether.home.HomeSources
import app.tether.home.ProjectGateway
import app.tether.home.createHomeSources
import app.tether.memory.MemoryAreaGateways
import app.tether.memory.createMemoryAreaGateways
import app.tether.messages.AppMessages
import app.tether.preview.InlinePreviews
import app.tether.preview.PreviewFilesGateway
import app.tether.projectarchive.ProjectArchiveGateway
import app.tether.projectarchive.createProjectArchiveGateway
import app.tether.providers.CoreProviderGateway
import app.tether.providers.ProviderGateway
import app.tether.session.CoreSessionGateway
import app.tether.session.QuietEvents
import app.tether.session.SessionGateway
import app.tether.session.SessionSettingsGateway
import app.tether.session.SessionStarter
import app.tether.session.createSessionSettingsGateway
import app.tether.settings.PrefsSettingsStore
import app.tether.settings.budgets.BudgetsGateway
import app.tether.settings.budgets.createBudgetsGateway
import app.tether.settings.createProjectDefaultsGateway
import app.tether.settings.executors.CoreExecutorsGateway
import app.tether.settings.executors.ExecutorsGateway
import app.tether.settings.github.GithubGateway
import app.tether.settings.github.createGithubGateway
import app.tether.settings.mcp.McpGateway
import app.tether.settings.mcp.createMcpGateway
import app.tether.share.ShareInbox
import app.tether.theme.PrefsThemeStore
import app.tether.theme.ThemeController
import app.tether.updates.CoreUpdateGateway
import app.tether.updates.PrefsUpdateStore
import app.tether.updates.UpdatesController
import app.tether.updates.createUpdateInstaller
import app.tether.updates.isDebuggableBuild
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.launch

/** Hand-wired dependencies. A DI library arrives only when this grows past a screenful. */
class AppModule(
    context: Context,
) {
    val core: CoreClient = CoreClient()
    val databasePath: String = context.getDatabasePath("tether.db").absolutePath
    val workspacesRoot: String =
        context.filesDir
            .resolve("workspaces")
            .apply { mkdirs() }
            .absolutePath
    val vault = keystoreSecretVault(context.filesDir.resolve("secrets"))
    val shareInbox = ShareInbox()

    /** The provider database opens on the first call, which runs off the main thread. */
    val providerGateway: ProviderGateway =
        CoreProviderGateway(
            lazy { ProviderSettings(context.getDatabasePath("tether.db").absolutePath, vault) },
        )

    /** The project list and creation. The runtime opens on the first call, off the main thread. */
    val projectGateway: ProjectGateway =
        CoreProjectGateway(
            lazy { ProjectRuntime(databasePath, workspacesRoot) },
        )

    /** Session lists and creation. The runtime opens on the first call, off the main thread. */
    val sessionGateway: SessionGateway by lazy { CoreSessionGateway(databasePath, vault) }

    /** Memory, skills, and vault for the Skills and memory screens. */
    val memoryArea: MemoryAreaGateways = createMemoryAreaGateways(context.filesDir)

    /** GitHub sign-in, the Git identity, and cloning. The runtime opens on the first call. */
    val githubGateway: GithubGateway by lazy {
        createGithubGateway(databasePath, workspacesRoot, vault)
    }

    /** Runtime packs, Linux mode, project network, and CI runs. Both runtimes open on first use. */
    val executorsGateway: ExecutorsGateway by lazy {
        CoreExecutorsGateway(
            sessionRuntime = lazy { SessionRuntime(databasePath, vault, QuietEvents) },
            ciRuntime = lazy { CiRuntime(databasePath, workspacesRoot, vault) },
        )
    }

    /** Budget limits and the cost summary. The runtime opens on the first call. */
    val budgetsGateway: BudgetsGateway by lazy { createBudgetsGateway(databasePath) }

    /** Importing a project from a ZIP and exporting one. The runtime opens on the first call. */
    val projectArchiveGateway: ProjectArchiveGateway by lazy {
        createProjectArchiveGateway(databasePath, workspacesRoot)
    }

    /** Git status, branches, history, and diffs of a project. The runtime opens on first use. */
    val gitGateway: GitGateway by lazy { createGitGateway(databasePath, workspacesRoot, vault) }

    /** MCP servers: the list, a server's tools, and adding one. */
    val mcpGateway: McpGateway by lazy { createMcpGateway(databasePath, vault) }

    /** Browsing, previewing, and searching a project's files. */
    val filesGateway: FilesGateway by lazy { createFilesGateway(databasePath, workspacesRoot) }

    /** The file bytes the preview reads. The runtime opens on the first read. */
    val previewFiles: PreviewFilesGateway by lazy {
        createPreviewFilesGateway(databasePath, workspacesRoot)
    }

    /** HTML of the inline previews opened from a session, held in memory only. */
    val inlinePreviews = InlinePreviews()

    /** Model, mode, rename, usage, rewind, and message search for the session screens. */
    val sessionSettings: SessionSettingsGateway by lazy {
        createSessionSettingsGateway(databasePath, vault)
    }

    /** Starts a session in a project, choosing the first provider that has a key. */
    val sessionStarter: SessionStarter by lazy {
        SessionStarter(
            providerGateway,
            sessionGateway,
            projectDefaults = createProjectDefaultsGateway(databasePath, workspacesRoot),
            appDefaults = PrefsSettingsStore(context),
        )
    }

    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    /** Announces that sessions were archived, restored, or deleted. The lists read again. */
    val sessionChanges: SessionChanges = SessionChanges()

    /**
     * Runs [block] on the app's own scope, off the main thread. It finishes even when the screen
     * that started it has gone.
     */
    fun launchInBackground(block: suspend () -> Unit) {
        appScope.launch(Dispatchers.IO) { block() }
    }

    /**
     * Runs [block] off the main thread, for native work such as closing a runtime. A failure is
     * logged by its class name and does not escape.
     */
    fun closeInBackground(block: () -> Unit) {
        appScope.launch(Dispatchers.IO) {
            runQuietly(block) { Log.w(TAG, "close failed: ${it.javaClass.simpleName}") }
        }
    }

    /** The data behind the home screens: projects and sessions. Debug and release builds differ. */
    val home: HomeSources = createHomeSources(projectGateway, sessionGateway)

    /** The update flow, shared by the Needs attention list and the About screen. */
    val updates: UpdatesController by lazy {
        UpdatesController(
            gateway =
                CoreUpdateGateway(
                    lazy { Updater(context.cacheDir.resolve("updates").absolutePath) },
                ),
            installer = createUpdateInstaller(context),
            store = PrefsUpdateStore(context),
            currentVersion =
                context.packageManager
                    .getPackageInfo(
                        context.packageName,
                        0,
                    ).versionName
                    .orEmpty(),
            scope = appScope,
            isDebuggable = isDebuggableBuild(context),
        )
    }

    /** The theme choice. It is saved in preferences and applies at once. */
    val theme = ThemeController(PrefsThemeStore(context))

    /** The theme choice, read by the activity. */
    val themeMode: StateFlow<ThemeMode> = theme.mode

    fun setThemeMode(mode: ThemeMode) {
        theme.set(mode)
    }

    /** The appearance choices: interface size, text size and accent. Saved at once. */
    val appearance = AppearanceController(PrefsAppearanceStore(context))

    /** Messages for the snackbar host, such as the toast after a test event. */
    val messages = AppMessages()
}

private const val TAG = "Tether"

/** Runs [block]. A failure goes to [onFailure] and does not escape. */
@Suppress("TooGenericExceptionCaught")
internal fun runQuietly(
    block: () -> Unit,
    onFailure: (Exception) -> Unit,
) {
    try {
        block()
    } catch (failure: Exception) {
        onFailure(failure)
    }
}

/**
 * A signal that sessions changed. Lists collect [changes] while they are on screen and read their
 * sessions again. A signal sent while no list is on screen is not kept; those lists read again
 * when they resume.
 */
class SessionChanges {
    private val events = MutableSharedFlow<Unit>(extraBufferCapacity = BUFFER)

    /** Fires once for each change. */
    val changes: SharedFlow<Unit> = events.asSharedFlow()

    /** Announces a change. It is safe to call from any thread. */
    fun notifySessionsChanged() {
        events.tryEmit(Unit)
    }

    private companion object {
        const val BUFFER = 8
    }
}
