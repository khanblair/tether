package app.tether.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.core.generated.ProviderView
import app.tether.providers.ProviderGateway
import app.tether.updates.UpdatesUiState
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import java.time.ZoneId
import java.util.Locale

/** What the dashboard loads from the sources. [providers] stays null until the keys are read. */
private data class HomeData(
    val projects: List<ProjectSummary>,
    val sessions: List<SessionSummary>,
    val usage: UsageSnapshot,
    val providers: List<ProviderReady>?,
)

/**
 * State holder for Home. The providers come from the Rust core, which blocks and opens the
 * database on first use, so [refresh] reads them on [io]. The update flow keeps Needs attention
 * current without a reload.
 */
class DashboardViewModel(
    private val sources: HomeSources,
    private val providers: ProviderGateway,
    updates: StateFlow<UpdatesUiState>,
    private val clock: () -> Long = System::currentTimeMillis,
    private val zone: ZoneId = ZoneId.systemDefault(),
    private val locale: Locale = Locale.getDefault(),
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val loaded =
        MutableStateFlow(HomeData(emptyList(), emptyList(), UsageSnapshot.Empty, null))

    /** The dashboard state. Providers are null until [refresh] has read them. */
    val state: StateFlow<DashboardUiState> =
        combine(updates, loaded) { update, data ->
            build(data, update)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            build(loaded.value, updates.value),
        )

    /**
     * Reads the saved keys and the in-memory sources again. Call it when Home shows. When the core
     * cannot read the keys, the Providers card stays hidden.
     */
    fun refresh() {
        viewModelScope.launch(io) {
            loaded.value =
                HomeData(
                    projects = sources.projects.projects(),
                    sessions = sources.sessions.sessions(),
                    usage = sources.usage.usage(),
                    providers = readProviders(),
                )
        }
    }

    /** The saved-key state of every provider, or null when the core cannot read it. */
    private fun readProviders(): List<ProviderReady>? =
        try {
            ProviderView.entries.map { ProviderReady(it, providers.hasKey(it)) }
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (
            @Suppress("TooGenericExceptionCaught", "SwallowedException") failure: Exception,
        ) {
            null
        }

    private fun build(
        data: HomeData,
        update: UpdatesUiState,
    ): DashboardUiState =
        dashboardState(
            projects = data.projects,
            sessions = data.sessions,
            usage = data.usage,
            updates = update,
            providers = data.providers,
            now = clock(),
            zone = zone,
            locale = locale,
        )

    init {
        refresh()
    }

    companion object {
        /** Builds the Home model from the app's sources, the provider gateway, and the updates. */
        fun factory(
            sources: HomeSources,
            providers: ProviderGateway,
            updates: StateFlow<UpdatesUiState>,
        ) = viewModelFactory {
            initializer { DashboardViewModel(sources, providers, updates) }
        }
    }
}
