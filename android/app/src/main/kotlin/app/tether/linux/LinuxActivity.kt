package app.tether.linux

import androidx.core.content.edit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

private const val LINUX_PREFERENCES = "linux"
private const val BACKGROUND_MODE_KEY = "background_mode"

/** The Linux activity that can keep child processes alive. */
data class LinuxActivityState(
    val shellOpen: Boolean,
    val runningProcesses: Int,
    val linuxReady: Boolean,
)

/** The setting that controls when the Linux foreground service is shown. */
enum class LinuxBackgroundMode {
    WhileRunning,
    WhenInstalled,
}

/** What the Linux notification reports. The Android boundary turns it into text. */
enum class LinuxNoticeKind {
    ShellOpen,
    Ready,
    Servers,
    ShellAndServers,
}

/** The notification state: a [kind] and the number of running [servers] it names. */
data class LinuxNotice(
    val kind: LinuxNoticeKind,
    val servers: Int = 0,
) {
    companion object {
        /** Rebuilds a notice from its stable kind name and count. Null for an unknown kind. */
        fun fromExtras(
            kindName: String?,
            servers: Int,
        ): LinuxNotice? =
            LinuxNoticeKind.entries
                .firstOrNull { it.name == kindName }
                ?.let { LinuxNotice(it, servers.coerceAtLeast(0)) }
    }
}

/** The service change requested by the Linux activity tracker. */
sealed interface ServiceAction {
    data object Stop : ServiceAction

    data class Run(
        val notice: LinuxNotice,
    ) : ServiceAction
}

/** Chooses the service state and notification for the current Linux activity. */
fun desiredService(
    state: LinuxActivityState,
    mode: LinuxBackgroundMode,
): ServiceAction {
    val running = state.shellOpen || state.runningProcesses > 0
    return when {
        mode == LinuxBackgroundMode.WhileRunning && !running -> ServiceAction.Stop
        mode == LinuxBackgroundMode.WhenInstalled && !state.linuxReady -> ServiceAction.Stop
        else -> ServiceAction.Run(activityNotice(state, mode))
    }
}

private fun activityNotice(
    state: LinuxActivityState,
    mode: LinuxBackgroundMode,
): LinuxNotice =
    when {
        state.shellOpen && state.runningProcesses > 0 ->
            LinuxNotice(LinuxNoticeKind.ShellAndServers, state.runningProcesses)
        state.runningProcesses > 0 -> LinuxNotice(LinuxNoticeKind.Servers, state.runningProcesses)
        mode == LinuxBackgroundMode.WhenInstalled -> LinuxNotice(LinuxNoticeKind.Ready)
        else -> LinuxNotice(LinuxNoticeKind.ShellOpen)
    }

/** Stores the mode through a small platform-free read and write seam. */
class LinuxBackgroundStore(
    private val read: () -> String?,
    private val write: (String) -> Unit,
) {
    /** Reads the saved mode, using [LinuxBackgroundMode.WhileRunning] for unknown values. */
    fun load(): LinuxBackgroundMode =
        LinuxBackgroundMode.entries.firstOrNull { it.name == read() }
            ?: LinuxBackgroundMode.WhileRunning

    /** Saves [mode] by its stable enum name. */
    fun save(mode: LinuxBackgroundMode) {
        write(mode.name)
    }
}

/** The app's saved Linux background setting. */
class LinuxBackgroundController(
    private val store: LinuxBackgroundStore,
) {
    private val mutableMode = MutableStateFlow(store.load())
    private val observers = mutableListOf<(LinuxBackgroundMode) -> Unit>()

    /** The current setting, loaded once when the app module is created. */
    val mode: StateFlow<LinuxBackgroundMode> = mutableMode.asStateFlow()

    /** Saves and publishes [mode]. */
    @Synchronized
    fun set(mode: LinuxBackgroundMode) {
        if (mutableMode.value == mode) return
        store.save(mode)
        mutableMode.value = mode
        observers.toList().forEach { observer -> observer(mode) }
    }

    /** Adds a listener and immediately gives it the current setting. */
    @Synchronized
    fun observe(observer: (LinuxBackgroundMode) -> Unit) {
        observers += observer
        observer(mutableMode.value)
    }
}

/** Creates the SharedPreferences-backed store required by the app. */
fun android.content.Context.linuxBackgroundStore(): LinuxBackgroundStore {
    val preferences = getSharedPreferences(LINUX_PREFERENCES, android.content.Context.MODE_PRIVATE)
    return LinuxBackgroundStore(
        read = { preferences.getString(BACKGROUND_MODE_KEY, null) },
        write = { mode -> preferences.edit { putString(BACKGROUND_MODE_KEY, mode) } },
    )
}

/** Platform boundary for the foreground service. */
interface LinuxServiceController {
    /**
     * Applies one desired service action. Returns false when a later foreground action must retry.
     */
    fun apply(action: ServiceAction): Boolean
}

/** Tracks shell and server sources and changes the service only when the desired state changes. */
class LinuxActivity(
    private val background: LinuxBackgroundController,
    private val service: LinuxServiceController,
) {
    private val sources = mutableMapOf<String, SourceState>()
    private val closedSources = mutableSetOf<String>()
    private var mode = background.mode.value
    private var linuxReady = false
    private var appVisible = false
    private var delivered: ServiceAction = ServiceAction.Stop
    private var pendingStart = false

    init {
        background.observe(::setMode)
    }

    /** Reports whether a shell source is open. */
    @Synchronized
    fun reportShell(
        sourceId: String,
        open: Boolean,
    ) {
        if (sourceId in closedSources) return
        updateSource(sourceId) { it.copy(shellOpen = open) }
    }

    /** Reports the running server count for one open session screen. */
    @Synchronized
    fun reportServers(
        sourceId: String,
        count: Int,
    ) {
        if (sourceId in closedSources) return
        updateSource(sourceId) { it.copy(runningProcesses = count.coerceAtLeast(0)) }
    }

    /** Marks a screen as open so a later report can use the same source id again. */
    @Synchronized
    fun openSource(sourceId: String) {
        closedSources.remove(sourceId)
        sources.putIfAbsent(sourceId, SourceState())
    }

    /** Drops a source when its screen closes. */
    @Synchronized
    fun removeSource(sourceId: String) {
        sources.remove(sourceId)
        closedSources += sourceId
        updateService()
    }

    /** Reports whether Linux setup is ready. */
    @Synchronized
    fun reportLinuxReady(ready: Boolean) {
        if (linuxReady == ready) return
        linuxReady = ready
        updateService()
    }

    /** Records the activity visibility used to gate foreground-service starts. */
    @Synchronized
    fun setAppVisible(visible: Boolean) {
        appVisible = visible
        if (visible && pendingStart) updateService()
    }

    private fun updateSource(
        sourceId: String,
        update: (SourceState) -> SourceState,
    ) {
        sources[sourceId] = update(sources[sourceId] ?: SourceState())
        updateService()
    }

    @Synchronized
    private fun setMode(nextMode: LinuxBackgroundMode) {
        mode = nextMode
        updateService()
    }

    @Synchronized
    private fun updateService() {
        val state =
            LinuxActivityState(
                shellOpen = sources.values.any { it.shellOpen },
                runningProcesses = sources.values.sumOf { it.runningProcesses },
                linuxReady = linuxReady,
            )
        val desired = desiredService(state, mode)
        when (desired) {
            ServiceAction.Stop -> {
                pendingStart = false
                if (delivered != desired) {
                    if (service.apply(desired)) delivered = desired
                }
            }
            is ServiceAction.Run -> {
                if (!appVisible) {
                    pendingStart = true
                } else if (delivered != desired || pendingStart) {
                    if (service.apply(desired)) {
                        delivered = desired
                        pendingStart = false
                    } else {
                        pendingStart = true
                    }
                }
            }
        }
    }

    private data class SourceState(
        val shellOpen: Boolean = false,
        val runningProcesses: Int = 0,
    )
}
