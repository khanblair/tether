package app.tether.design.components

import androidx.compose.runtime.Immutable

/** One figure in a [StatRow]: the [value], its [label], and a plain [comparison] line. */
@Immutable
data class StatItem(
    val value: String,
    val label: String,
    val comparison: String,
)
