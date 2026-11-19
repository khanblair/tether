package app.tether.providers

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderView
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * State holder for the provider list. It reads each provider's saved status and never tests a
 * key.
 */
class ProvidersViewModel(
    private val gateway: ProviderGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(ProvidersUiState())

    /** The list state. */
    val state: StateFlow<ProvidersUiState> = mutableState.asStateFlow()

    init {
        refresh()
    }

    /** Reads every provider again. Call it when the screen comes back from a detail screen. */
    fun refresh() {
        viewModelScope.launch {
            val rows = withContext(io) { ProviderView.entries.map { it to read(it) } }
            mutableState.update {
                ProvidersUiState(
                    items = rows.map { (provider, row) -> row ?: ProviderSummary(provider) },
                    loading = false,
                    failed = rows.any { (_, row) -> row == null },
                )
            }
        }
    }

    private fun read(provider: ProviderView): ProviderSummary? =
        try {
            val hasKey = gateway.hasKey(provider)
            val saved = gateway.status(provider)
            ProviderSummary(
                provider = provider,
                hasKey = hasKey,
                status = if (hasKey) saved.status else ConnectionStatusView.UNSET,
                modelCount = saved.modelCount.toInt(),
                testedAtMs = saved.testedAtMs,
            )
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (
            @Suppress("TooGenericExceptionCaught", "SwallowedException") failure: Exception,
        ) {
            null
        }

    companion object {
        /** Builds the list model with the app's gateway. */
        fun factory(gateway: ProviderGateway) =
            viewModelFactory { initializer { ProvidersViewModel(gateway) } }
    }
}
