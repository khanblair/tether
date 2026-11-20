package app.tether.session

import android.database.sqlite.SQLiteDatabase
import java.util.Locale

private const val PERCENT = 100L
private const val MICROS_PER_DOLLAR = 1_000_000.0

/** Saved usage and the selected model. A missing catalog context stays unknown. */
data class SessionStatus(
    val modelId: String,
    val mode: String,
    val tokensIn: Long,
    val tokensOut: Long,
    val costMicros: Long,
    val contextWindow: Long?,
) {
    val contextPercent: Int?
        get() = contextWindow?.takeIf { it > 0 }?.let { window ->
            ((tokensIn * PERCENT) / window).coerceIn(0, PERCENT).toInt()
        }

    fun costText(): String = String.format(Locale.US, "%.2f", costMicros / MICROS_PER_DOLLAR)
}

/** Reads a snapshot without writing to the database owned by the Rust actor. */
internal fun readSessionStatus(databasePath: String, sessionId: String): SessionStatus {
    val db = SQLiteDatabase.openDatabase(databasePath, null, SQLiteDatabase.OPEN_READONLY)
    db.use { database ->
        val sql = """SELECT s.model_id, s.mode, s.tokens_in, s.tokens_out, s.cost_micros,
            m.context_window FROM sessions s LEFT JOIN models m
            ON m.provider = s.provider AND m.model_id = s.model_id WHERE s.id = ?"""
        database.rawQuery(sql, arrayOf(sessionId)).use { cursor ->
            check(cursor.moveToFirst()) { "session not found" }
            return SessionStatus(
                modelId = cursor.getString(0),
                mode = cursor.getString(1),
                tokensIn = cursor.getLong(2),
                tokensOut = cursor.getLong(3),
                costMicros = cursor.getLong(4),
                contextWindow = if (cursor.isNull(5)) null else cursor.getLong(5),
            )
        }
    }
}
