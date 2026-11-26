package app.tether.preview

/**
 * HTML held in memory for an inline preview, so the route carries only an id. It keeps the last
 * [capacity] entries and nothing is saved. Call it on the main thread.
 */
class InlinePreviews(
    private val capacity: Int = INLINE_PREVIEW_CAPACITY,
) {
    private val entries = LinkedHashMap<String, String>()
    private var counter = 0L

    /** Holds [html] and returns its id. The oldest entry goes when the capacity is passed. */
    fun put(html: String): String {
        counter += 1
        val id = "inline-$counter"
        entries[id] = html
        while (entries.size > capacity) {
            entries.remove(entries.keys.first())
        }
        return id
    }

    /** The HTML for [id], or null when it is unknown or has been dropped. */
    fun get(id: String): String? = entries[id]
}

/** How many inline previews are kept. */
const val INLINE_PREVIEW_CAPACITY = 8
