package app.tether.session.voice

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** How many bars the waveform draws. */
const val VOICE_BARS = 10

private const val RMS_MIN_DB = -2f
private const val RMS_MAX_DB = 10f
private val MIN_BAR: Dp = 8.dp
private val MAX_BAR: Dp = 40.dp

/** Maps the recognizer's RMS value (about -2 to 10 dB) to a level from 0 to 1. */
fun levelFromRms(rmsDb: Float): Float =
    ((rmsDb - RMS_MIN_DB) / (RMS_MAX_DB - RMS_MIN_DB)).coerceIn(0f, 1f)

/** Shifts [level] in at the end and drops the oldest entry. The list always has [bars] entries. */
fun nextLevels(
    previous: List<Float>,
    level: Float,
    bars: Int = VOICE_BARS,
): List<Float> {
    val kept = previous.takeLast(bars - 1)
    val padding = List(bars - kept.size - 1) { 0f }
    return padding + kept + level.coerceIn(0f, 1f)
}

/** The bar height for a level: 8 dp when silent, 40 dp when loud. */
fun barHeight(level: Float): Dp = MIN_BAR + (MAX_BAR - MIN_BAR) * level.coerceIn(0f, 1f)
