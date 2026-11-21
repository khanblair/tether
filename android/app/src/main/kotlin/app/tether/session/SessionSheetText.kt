package app.tether.session

import java.util.Locale

private const val MICROS_PER_DOLLAR = 1_000_000.0

/**
 * A price in dollars per million tokens, from micros: "$3", "$1.25", or "$0.5". Locale.US keeps the
 * decimal point the same on every device.
 */
fun formatPricePerMillion(micros: Long): String =
    "$" + String.format(Locale.US, "%.2f", micros / MICROS_PER_DOLLAR).trimEnd('0').trimEnd('.')

/** The session cost in dollars, or [unspent] while nothing has been spent. */
fun costLabel(
    costMicros: Long,
    unspent: String,
): String =
    if (costMicros <= 0L) {
        unspent
    } else {
        "$" + String.format(Locale.US, "%.2f", costMicros / MICROS_PER_DOLLAR)
    }
