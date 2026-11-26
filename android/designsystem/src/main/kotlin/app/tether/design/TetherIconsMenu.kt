// Lucide icons for the session menu, taken from tools/prototype/icons.js (ISC license,
// https://lucide.dev). Kept apart from TetherIcons.kt, which is at the 400-line limit.
// Numeric coordinates are icon geometry, so MagicNumber is suppressed for this file.
@file:Suppress("MagicNumber")

package app.tether.design

import androidx.compose.ui.graphics.vector.ImageVector

/** A pencil, for Rename. */
val TetherIcons.Pencil: ImageVector by lazy {
    tetherIcon("Pencil") {
        strokeSvg(
            "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352" +
                "a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
        )
        strokeSvg("m15 5 4 4")
    }
}

/** Three linked dots, for Export as Markdown. */
val TetherIcons.Share2: ImageVector by lazy {
    tetherIcon("Share2") {
        strokePath { circle(18f, 5f, 3f) }
        strokePath { circle(6f, 12f, 3f) }
        strokePath { circle(18f, 19f, 3f) }
        strokeSvg("M8.59 13.51L15.42 17.49")
        strokeSvg("M15.41 6.51L8.59 10.49")
    }
}

/** A clock with a back arrow, for Checkpoints. */
val TetherIcons.History: ImageVector by lazy {
    tetherIcon("History") {
        strokeSvg("M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8")
        strokeSvg("M3 3v5h5")
        strokeSvg("M12 7v5l4 2")
    }
}

/** A filing box, for Archive. */
val TetherIcons.Archive: ImageVector by lazy {
    tetherIcon("Archive") {
        strokeSvg("M3 3h18a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z")
        strokeSvg("M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8")
        strokeSvg("M10 12h4")
    }
}

/** A bin, for Delete. */
val TetherIcons.Trash2: ImageVector by lazy {
    tetherIcon("Trash2") {
        strokeSvg("M10 11v6")
        strokeSvg("M14 11v6")
        strokeSvg("M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6")
        strokeSvg("M3 6h18")
        strokeSvg("M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2")
    }
}
