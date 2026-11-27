// Lucide icon for the Appearance setting, taken from tools/prototype/icons.js (ISC license,
// https://lucide.dev). Kept apart from TetherIcons.kt, which is at the 400-line limit.
// Numeric coordinates are icon geometry, so MagicNumber is suppressed for this file.
@file:Suppress("MagicNumber")

package app.tether.design

import androidx.compose.ui.graphics.vector.ImageVector

/** A palette symbol for the Appearance setting. */
val TetherIcons.Palette: ImageVector by lazy {
    tetherIcon("Palette") {
        strokeSvg(
            "M12 22a1 1 0 0 1 0-20 10 9 0 0 1 10 9 5 5 0 0 1-5 5h-2.25a1.75 1.75 " +
                "0 0 0-1.4 2.8l.3.4a1.75 1.75 0 0 1-1.4 2.8z",
        )
        strokePath { circle(13.5f, 6.5f, 0.5f) }
        strokePath { circle(17.5f, 10.5f, 0.5f) }
        strokePath { circle(6.5f, 12.5f, 0.5f) }
        strokePath { circle(8.5f, 7.5f, 0.5f) }
    }
}
