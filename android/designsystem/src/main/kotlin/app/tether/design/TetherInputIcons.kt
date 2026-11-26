// Icon geometry for the session composer, from Lucide (ISC license, https://lucide.dev), taken from
// tools/prototype/icons.js. Kept apart from TetherIcons.kt, which is at the 400-line file limit.
// Numeric coordinates are icon geometry, so MagicNumber is suppressed for this file.
@file:Suppress("MagicNumber")

package app.tether.design

import androidx.compose.ui.graphics.vector.ImageVector

/** Lucide icons for the composer, drawn in solid black so callers tint them. */
object TetherInputIcons {
    val Shield: ImageVector by lazy {
        tetherIcon("Shield") {
            strokeSvg(SHIELD_PATH)
        }
    }

    val ShieldAlert: ImageVector by lazy {
        tetherIcon("ShieldAlert") {
            strokeSvg(SHIELD_PATH)
            strokeSvg("M12 8v4")
            strokeSvg("M12 16h.01")
        }
    }

    val Mic: ImageVector by lazy {
        tetherIcon("Mic") {
            strokeSvg("M12 19v3")
            strokeSvg("M19 10v2a7 7 0 0 1-14 0v-2")
            strokeSvg("M9 5a3 3 0 0 1 6 0v7a3 3 0 0 1-6 0z")
        }
    }

    val ArrowUp: ImageVector by lazy {
        tetherIcon("ArrowUp") {
            strokeSvg("m5 12 7-7 7 7")
            strokeSvg("M12 19V5")
        }
    }

    val Square: ImageVector by lazy {
        tetherIcon("Square") {
            strokeSvg(ROUNDED_SQUARE_PATH)
        }
    }

    val Image: ImageVector by lazy {
        tetherIcon("Image") {
            strokeSvg(ROUNDED_SQUARE_PATH)
            strokePath { circle(9f, 9f, 2f) }
            strokeSvg("m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21")
        }
    }

    // Not in tools/prototype/icons.js: the standard Lucide camera path.
    val Camera: ImageVector by lazy {
        tetherIcon("Camera") {
            strokeSvg(
                "M13.997 4a2 2 0 0 1 1.76 1.05l.486.9A2 2 0 0 0 18.003 7H20a2 2 0 0 1 2 2v9" +
                    "a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h1.997a2 2 0 0 0 1.75-1.05" +
                    "l.486-.9A2 2 0 0 1 10.003 4z",
            )
            strokePath { circle(12f, 13f, 3f) }
        }
    }

    val Paperclip: ImageVector by lazy {
        tetherIcon("Paperclip") {
            strokeSvg(
                "m16 6-8.414 8.586a2 2 0 0 0 2.829 2.829l8.414-8.586a4 4 0 1 0-5.657-5.657" +
                    "l-8.379 8.551a6 6 0 1 0 8.485 8.485l8.379-8.551",
            )
        }
    }

    val AtSign: ImageVector by lazy {
        tetherIcon("AtSign") {
            strokePath { circle(12f, 12f, 4f) }
            strokeSvg("M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-4 8")
        }
    }

    val SquareSlash: ImageVector by lazy {
        tetherIcon("SquareSlash") {
            strokeSvg(ROUNDED_SQUARE_PATH)
            strokeSvg("M9 15 15 9")
        }
    }

    private const val SHIELD_PATH =
        "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1" +
            "c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"
    private const val ROUNDED_SQUARE_PATH =
        "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"
}
