package app.tether.design.components

/**
 * Turns bar values into heights from 0 to 1, each over the largest value. Negative values count
 * as zero, and when every value is zero (or none is positive) every bar is 0.
 */
internal fun barFractions(values: List<Long>): List<Float> {
    val max = values.maxOrNull() ?: 0L
    if (max <= 0L) return values.map { 0f }
    return values.map { value ->
        (value.coerceAtLeast(0L).toDouble() / max.toDouble()).toFloat()
    }
}
