// Icon geometry from Lucide (ISC license, https://lucide.dev), converted to Compose ImageVector.
// Source: tools/prototype/icons.js. Grid 24 dp, stroke width 1.75, round caps and joins.
// Numeric coordinates are icon geometry, so MagicNumber is suppressed for this file.
@file:Suppress("MagicNumber")

package app.tether.design

import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.PathBuilder
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.unit.dp

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
}

private const val STROKE_WIDTH = 1.75f
private val INK: Brush = SolidColor(Color.Black)

private fun tetherIcon(
    name: String,
    build: ImageVector.Builder.() -> Unit,
): ImageVector =
    ImageVector
        .Builder(
            name = name,
            defaultWidth = 24.dp,
            defaultHeight = 24.dp,
            viewportWidth = 24f,
            viewportHeight = 24f,
        ).apply(build)
        .build()

private fun ImageVector.Builder.strokePath(
    pathBuilder: PathBuilder.() -> Unit,
): ImageVector.Builder =
    path(
        fill = null,
        stroke = INK,
        strokeLineWidth = STROKE_WIDTH,
        strokeLineCap = StrokeCap.Round,
        strokeLineJoin = StrokeJoin.Round,
        pathBuilder = pathBuilder,
    )

private fun ImageVector.Builder.strokeSvg(pathData: String): ImageVector.Builder =
    addPath(
        pathData = addPathNodes(pathData),
        fill = null,
        stroke = INK,
        strokeLineWidth = STROKE_WIDTH,
        strokeLineCap = StrokeCap.Round,
        strokeLineJoin = StrokeJoin.Round,
    )

/** A full circle as two half arcs, so it stays one closed subpath. */
private fun PathBuilder.circle(
    cx: Float,
    cy: Float,
    r: Float,
) {
    moveTo(cx - r, cy)
    arcTo(r, r, 0f, false, true, cx + r, cy)
    arcTo(r, r, 0f, false, true, cx - r, cy)
    close()
}
