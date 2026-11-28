package app.tether.session.loads

/** The count bucket used to choose a pluralized detail line. */
internal enum class DetailCount {
    None,
    One,
    Many,
}

/** Chooses the detail-line bucket for a non-negative count. */
internal fun detailCount(count: Int): DetailCount =
    when (count) {
        0 -> DetailCount.None
        1 -> DetailCount.One
        else -> DetailCount.Many
    }

/** The instructions row has a file detail or a no-file detail. */
internal fun instructionsDetail(file: String?): Boolean = file != null
