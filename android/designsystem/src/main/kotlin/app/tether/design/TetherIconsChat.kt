// Lucide icons for the chat message actions, taken from tools/prototype/icons.js (ISC license,
// https://lucide.dev). Kept apart from TetherIcons.kt, which is at the 400-line limit.
// Numeric coordinates are icon geometry, so MagicNumber is suppressed for this file.
@file:Suppress("MagicNumber")

package app.tether.design

import androidx.compose.ui.graphics.vector.ImageVector

/** Two overlapping sheets, for Copy. */
val TetherIcons.Copy: ImageVector by lazy {
    tetherIcon("Copy") {
        strokeSvg(
            "M10 8h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H10a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z",
        )
        strokeSvg("M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2")
    }
}

/** Two circling arrows, for Regenerate. */
val TetherIcons.RefreshCw: ImageVector by lazy {
    tetherIcon("RefreshCw") {
        strokeSvg("M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8")
        strokeSvg("M21 3v5h-5")
        strokeSvg("M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16")
        strokeSvg("M8 16H3v5")
    }
}
