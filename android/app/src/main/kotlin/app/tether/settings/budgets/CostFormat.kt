package app.tether.settings.budgets

import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

/** The device's current UTC offset in minutes, for the core's display day. */
fun currentUtcOffsetMinutes(): Int =
    ZoneId
        .systemDefault()
        .rules
        .getOffset(Instant.now())
        .totalSeconds / SECONDS_PER_MINUTE

/** A `YYYY-MM-DD` day as a short label such as "5 Nov". */
fun dayLabel(
    day: String,
    locale: Locale,
): String = LocalDate.parse(day).format(DateTimeFormatter.ofPattern("d MMM", locale))

private const val SECONDS_PER_MINUTE = 60
