package app.tether.settings.budgets

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.util.concurrent.atomic.AtomicInteger

private const val SHORT_RANGE_DAYS = 7
private const val LONG_RANGE_DAYS = 30

/** The two windows the chart and tables can show. */
enum class CostRange(
    val days: Int,
) {
    /** The last seven days. */
    Week(SHORT_RANGE_DAYS),

    /** The last thirty days. */
    Month(LONG_RANGE_DAYS),
}

/**
 * What the cost screen shows. [summary] is null until the first load succeeds. [spentInRange]
 * sums the chart's days, and [empty] is true when that sum is zero.
 */
@Immutable
data class CostUiState(
    val range: CostRange = CostRange.Week,
    val loading: Boolean = true,
    val failed: Boolean = false,
    val summary: CostSummary? = null,
) {
    /** The spend over the days of the selected range. */
    val spentInRange: Long
        get() = summary?.byDay?.sumOf { it.costMicros } ?: 0L

    /** True when the summary loaded and no day in the range has spend. */
    val empty: Boolean
        get() = summary != null && spentInRange == 0L
}

/**
 * State for the cost screen. Each load passes the device's UTC offset from [utcOffsetMinutes].
 * A load whose range was replaced by a newer one drops its result.
 */
class CostViewModel(
    private val gateway: BudgetsGateway,
    private val utcOffsetMinutes: () -> Int = ::currentUtcOffsetMinutes,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(CostUiState())

    /** The screen state. */
    val state: StateFlow<CostUiState> = mutableState.asStateFlow()

    /** Counts loads, so a stale load's result is dropped. */
    private val generation = AtomicInteger(0)

    init {
        load()
    }

    /** Shows [range]. A different range reloads the summary. */
    fun select(range: CostRange) {
        if (range == state.value.range) return
        mutableState.update { it.copy(range = range, loading = true, failed = false) }
        load()
    }

    /** Loads the summary again after a failure. */
    fun retry() {
        mutableState.update { it.copy(loading = true, failed = false) }
        load()
    }

    private fun load() {
        val token = generation.incrementAndGet()
        val range = state.value.range
        val offset = utcOffsetMinutes()
        viewModelScope.launch {
            val result = callCore(io) { gateway.usageSummary(range.days, offset) }
            if (token != generation.get()) return@launch
            result
                .onSuccess { summary ->
                    mutableState.update {
                        it.copy(loading = false, failed = false, summary = summary)
                    }
                }.onFailure {
                    mutableState.update { it.copy(loading = false, failed = true) }
                }
        }
    }
}
