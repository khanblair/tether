// Lucide icon for the preview network chip, taken from tools/prototype/icons.js (ISC license,
// https://lucide.dev). Kept apart from TetherIcons.kt, which is at the 400-line limit.
// Numeric coordinates are icon geometry, so MagicNumber is suppressed for this file.
@file:Suppress("MagicNumber")

package app.tether.design

import androidx.compose.ui.graphics.vector.ImageVector

/** A wifi symbol with a slash, for the preview network chip while the network is off. */
val TetherIcons.WifiOff: ImageVector by lazy {
    tetherIcon("WifiOff") {
        strokeSvg("M12 20h.01")
        strokeSvg("M8.5 16.429a5 5 0 0 1 7 0")
        strokeSvg("M5 12.859a10 10 0 0 1 5.17-2.69")
        strokeSvg("M19 12.859a10 10 0 0 0-2.007-1.523")
        strokeSvg("M2 8.82a15 15 0 0 1 4.177-2.643")
        strokeSvg("M22 8.82a15 15 0 0 0-11.288-3.764")
        strokeSvg("m2 2 20 20")
    }
}
