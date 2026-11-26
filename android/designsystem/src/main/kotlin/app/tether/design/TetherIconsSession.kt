// Lucide icons for the session top bar and the jump-to-latest pill, taken from
// tools/prototype/icons.js (ISC license, https://lucide.dev). Kept apart from TetherIcons.kt,
// which is at the 400-line limit.
// Numeric coordinates are icon geometry, so MagicNumber is suppressed for this file.
@file:Suppress("MagicNumber")

package app.tether.design

import androidx.compose.ui.graphics.vector.ImageVector

private const val FILE_DIFF_FRAME =
    "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588" +
        "A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"

/** A file with a diff mark, for "Review changes". Drawn in solid black so callers tint it. */
val TetherIcons.FileDiff: ImageVector by lazy {
    tetherIcon("FileDiff") {
        strokeSvg(FILE_DIFF_FRAME)
        strokeSvg("M9 10h6")
        strokeSvg("M12 13V7")
        strokeSvg("M9 17h6")
    }
}

/** Three dots in a column, for the session menu. */
val TetherIcons.EllipsisVertical: ImageVector by lazy {
    tetherIcon("EllipsisVertical") {
        strokePath { circle(12f, 12f, 1f) }
        strokePath { circle(12f, 5f, 1f) }
        strokePath { circle(12f, 19f, 1f) }
    }
}

/** A down arrow, for the jump-to-latest pill. */
val TetherIcons.ArrowDown: ImageVector by lazy {
    tetherIcon("ArrowDown") {
        strokeSvg("M12 5v14")
        strokeSvg("m19 12-7 7-7-7")
    }
}
