// Icon geometry from Lucide (ISC license, https://lucide.dev), converted to Compose ImageVector.
// Source: tools/prototype/icons.js. Grid 24 dp, stroke width 1.75, round caps and joins.
// Numeric coordinates are icon geometry, so MagicNumber is suppressed for this file.
@file:Suppress("MagicNumber")

package app.tether.design

import androidx.compose.ui.graphics.vector.ImageVector

/** Lucide icons drawn in solid black so callers tint them. Build each one once. */
object TetherIcons {
    val CircleCheck: ImageVector by lazy {
        tetherIcon("CircleCheck") {
            strokePath { circle(12f, 12f, 10f) }
            strokeSvg("M16 9l-5.5 5.5L8 12")
        }
    }

    val CircleX: ImageVector by lazy {
        tetherIcon("CircleX") {
            strokePath { circle(12f, 12f, 10f) }
            strokeSvg("M15 9l-6 6M9 9l6 6")
        }
    }

    val CircleDot: ImageVector by lazy {
        tetherIcon("CircleDot") {
            strokePath {
                circle(12f, 12f, 1f)
                circle(12f, 12f, 10f)
            }
        }
    }

    val ArrowLeft: ImageVector by lazy {
        tetherIcon("ArrowLeft") {
            strokePath {
                moveTo(12f, 19f)
                lineTo(5f, 12f)
                lineTo(12f, 5f)
                moveTo(19f, 12f)
                lineTo(5f, 12f)
            }
        }
    }

    val ChevronRight: ImageVector by lazy {
        tetherIcon("ChevronRight") {
            strokePath {
                moveTo(9f, 18f)
                lineTo(15f, 12f)
                lineTo(9f, 6f)
            }
        }
    }

    val ChevronLeft: ImageVector by lazy {
        tetherIcon("ChevronLeft") {
            strokePath {
                moveTo(15f, 18f)
                lineTo(9f, 12f)
                lineTo(15f, 6f)
            }
        }
    }

    val Zap: ImageVector by lazy {
        tetherIcon("Zap") {
            strokeSvg(
                "M15.914 4a1.5 1.5 0 0 0 -2.474 -1.561l-9 9A1.5 1.5 0 0 0 5.5 14h4.002" +
                    "a.5 .5 0 0 1 .471 .666L8.086 20a1.5 1.5 0 0 0 2.475 1.56l9-9" +
                    "A1.5 1.5 0 0 0 18.5 10h-3.997a.5 .5 0 0 1 -.472 -.667z",
            )
        }
    }

    val Trash2: ImageVector by lazy {
        tetherIcon("Trash2") {
            strokePath {
                moveTo(10f, 11f)
                lineTo(10f, 17f)
                moveTo(14f, 11f)
                lineTo(14f, 17f)
                moveTo(3f, 6f)
                lineTo(21f, 6f)
            }
            strokeSvg(
                "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" +
                    "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",
            )
        }
    }

    val Eye: ImageVector by lazy {
        tetherIcon("Eye") {
            strokeSvg(
                "M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0" +
                    "a1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0",
            )
            strokePath { circle(12f, 12f, 3f) }
        }
    }

    val EyeOff: ImageVector by lazy {
        tetherIcon("EyeOff") {
            strokeSvg(
                "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575a1 1 0 0 1 0 .696" +
                    "a10.747 10.747 0 0 1-1.444 2.49" +
                    "M14.084 14.158a3 3 0 0 1-4.242-4.242" +
                    "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151a1 1 0 0 1 0-.696" +
                    "a10.75 10.75 0 0 1 4.446-5.143" +
                    "M2 2l20 20",
            )
        }
    }

    val KeyRound: ImageVector by lazy {
        tetherIcon("KeyRound") {
            strokeSvg(
                "M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1" +
                    "a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586" +
                    "l.814-.814a6.5 6.5 0 1 0-4-4z",
            )
            strokePath { circle(16.5f, 7.5f, 0.5f) }
        }
    }

    val Menu: ImageVector by lazy {
        tetherIcon("Menu") {
            strokePath {
                moveTo(4f, 5f)
                lineTo(20f, 5f)
                moveTo(4f, 12f)
                lineTo(20f, 12f)
                moveTo(4f, 19f)
                lineTo(20f, 19f)
            }
        }
    }

    val Search: ImageVector by lazy {
        tetherIcon("Search") {
            strokeSvg("m21 21-4.34-4.34")
            strokePath { circle(11f, 11f, 8f) }
        }
    }

    val Bell: ImageVector by lazy {
        tetherIcon("Bell") {
            strokeSvg("M10.268 21a2 2 0 0 0 3.464 0")
            strokeSvg(
                "M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673" +
                    "C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8" +
                    "c0 4.499-1.411 5.956-2.738 7.326",
            )
        }
    }

    val Settings: ImageVector by lazy {
        tetherIcon("Settings") {
            strokeSvg(
                "M9.671 4.136a2.34 2.34 0 0 1 4.659 0a2.34 2.34 0 0 0 3.319 1.915" +
                    "a2.34 2.34 0 0 1 2.33 4.033a2.34 2.34 0 0 0 0 3.831" +
                    "a2.34 2.34 0 0 1-2.33 4.033a2.34 2.34 0 0 0-3.319 1.915" +
                    "a2.34 2.34 0 0 1-4.659 0a2.34 2.34 0 0 0-3.32-1.915" +
                    "a2.34 2.34 0 0 1-2.33-4.033a2.34 2.34 0 0 0 0-3.831" +
                    "A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915",
            )
            strokePath { circle(12f, 12f, 3f) }
        }
    }

    val Folder: ImageVector by lazy {
        tetherIcon("Folder") {
            strokeSvg(
                "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9" +
                    "A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z",
            )
        }
    }

    val FileText: ImageVector by lazy {
        tetherIcon("FileText") {
            strokeSvg(
                "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" +
                    "M14 2v4a2 2 0 0 0 2 2h4M10 9H8M16 13H8M16 17H8",
            )
        }
    }

    val FolderOpen: ImageVector by lazy {
        tetherIcon("FolderOpen") {
            strokeSvg(
                "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6" +
                    "a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9" +
                    "l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2",
            )
        }
    }

    val GitBranch: ImageVector by lazy {
        tetherIcon("GitBranch") {
            strokeSvg("M15 6a9 9 0 0 0-9 9V3")
            strokePath {
                circle(18f, 6f, 3f)
                circle(6f, 18f, 3f)
            }
        }
    }

    val Package: ImageVector by lazy {
        tetherIcon("Package") {
            strokeSvg(
                "M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73" +
                    "l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z",
            )
            strokeSvg("M12 22V12")
            strokePath {
                moveTo(3.29f, 7f)
                lineTo(12f, 12f)
                lineTo(20.71f, 7f)
            }
            strokeSvg("m7.5 4.27 9 5.15")
        }
    }

    val Plus: ImageVector by lazy {
        tetherIcon("Plus") {
            strokeSvg("M5 12h14")
            strokeSvg("M12 5v14")
        }
    }

    val Brain: ImageVector by lazy {
        tetherIcon("Brain") {
            strokeSvg("M12 18V5")
            strokeSvg("M15 13a4.17 4.17 0 0 1-3-4 4.17 4.17 0 0 1-3 4")
            strokeSvg("M17.707 6.294A3 3 0 1 0 12 5a3 3 0 1 0-5.707 1.294")
            strokeSvg("M17.996 5.125a4 4 0 0 1 2.526 5.77")
            strokeSvg("M18 18a4 4 0 0 0 2-7.465")
            strokeSvg("M19.967 17.482a4 4 0 1 1-7.967.517 4 4 0 1 1-7.967-.517")
            strokeSvg("M6 18a4 4 0 0 1-2-7.465")
            strokeSvg("M6.004 5.125a4 4 0 0 0-2.526 5.77")
        }
    }

    val Loader: ImageVector by lazy {
        tetherIcon("Loader") {
            strokeSvg("M12 2v4")
            strokeSvg("m16.2 7.8 2.9-2.9")
            strokeSvg("M18 12h4")
            strokeSvg("m16.2 16.2 2.9 2.9")
            strokeSvg("M12 18v4")
            strokeSvg("m4.9 19.1 2.9-2.9")
            strokeSvg("M2 12h4")
            strokeSvg("m4.9 4.9 2.9 2.9")
        }
    }

    val CirclePause: ImageVector by lazy {
        tetherIcon("CirclePause") {
            strokePath {
                circle(12f, 12f, 10f)
                moveTo(10f, 15f)
                lineTo(10f, 9f)
                moveTo(14f, 15f)
                lineTo(14f, 9f)
            }
        }
    }

    val X: ImageVector by lazy {
        tetherIcon("X") {
            strokeSvg("M18 6 6 18")
            strokeSvg("m6 6 12 12")
        }
    }

    val House: ImageVector by lazy {
        tetherIcon("House") {
            strokeSvg("M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8")
            strokeSvg(
                "M3 10a2 2 0 0 1 .709-1.528l7-5.999a2 2 0 0 1 2.582 0l7 5.999" +
                    "A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
            )
        }
    }

    val Sun: ImageVector by lazy {
        tetherIcon("Sun") {
            strokePath { circle(12f, 12f, 4f) }
            strokeSvg("M12 2v2")
            strokeSvg("M12 20v2")
            strokeSvg("m4.93 4.93 1.41 1.41")
            strokeSvg("m17.66 17.66 1.41 1.41")
            strokeSvg("M2 12h2")
            strokeSvg("M20 12h2")
            strokeSvg("m6.34 17.66-1.41 1.41")
            strokeSvg("m19.07 4.93-1.41 1.41")
        }
    }

    val Moon: ImageVector by lazy {
        tetherIcon("Moon") {
            strokeSvg(
                "M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803" +
                    "a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401",
            )
        }
    }

    val Monitor: ImageVector by lazy {
        tetherIcon("Monitor") {
            strokeSvg("M2 5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z")
            strokeSvg("M8 21h8")
            strokeSvg("M12 17v4")
        }
    }

    val ChartColumn: ImageVector by lazy {
        tetherIcon("ChartColumn") {
            strokeSvg("M3 3v16a2 2 0 0 0 2 2h16")
            strokeSvg("M18 17V9")
            strokeSvg("M13 17V5")
            strokeSvg("M8 17v-3")
        }
    }

    val Activity: ImageVector by lazy {
        tetherIcon("Activity") {
            strokeSvg(
                "M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0" +
                    "L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2",
            )
        }
    }

    val ChevronDown: ImageVector by lazy {
        tetherIcon("ChevronDown") {
            strokeSvg("m6 9 6 6 6-6")
        }
    }

    val ChevronUp: ImageVector by lazy {
        tetherIcon("ChevronUp") {
            strokeSvg("m18 15-6-6-6 6")
        }
    }

    val Terminal: ImageVector by lazy {
        tetherIcon("Terminal") {
            strokeSvg("M12 19h8")
            strokeSvg("m4 17 6-6-6-6")
        }
    }

    val FilePen: ImageVector by lazy {
        tetherIcon("FilePen") {
            strokeSvg(
                "M12.659 22H18a2 2 0 0 0 2-2V8a2.4 2.4 0 0 0-.706-1.706" +
                    "l-3.588-3.588A2.4 2.4 0 0 0 14 2H6a2 2 0 0 0-2 2v9.34",
            )
            strokeSvg("M14 2v5a1 1 0 0 0 1 1h5")
            strokeSvg(
                "M10.378 12.622a1 1 0 0 1 3 3.003L8.36 20.637a2 2 0 0 1-.854.506" +
                    "l-2.867.837a.5.5 0 0 1-.62-.62l.836-2.869a2 2 0 0 1 .506-.853z",
            )
        }
    }

    val Upload: ImageVector by lazy {
        tetherIcon("Upload") {
            strokeSvg("M12 3v12")
            strokeSvg("m17 8-5-5-5 5")
            strokeSvg("M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4")
        }
    }

    val Globe: ImageVector by lazy {
        tetherIcon("Globe") {
            strokePath { circle(12f, 12f, 10f) }
            strokeSvg("M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20")
            strokeSvg("M2 12h20")
        }
    }
}
