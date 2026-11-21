// Builders for the icon geometry in TetherIcons.kt: a 24 dp ImageVector with round-capped strokes.
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

private const val STROKE_WIDTH = 1.75f
private val INK: Brush = SolidColor(Color.Black)

internal fun tetherIcon(
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

internal fun ImageVector.Builder.strokePath(
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

internal fun ImageVector.Builder.strokeSvg(pathData: String): ImageVector.Builder =
    addPath(
        pathData = addPathNodes(pathData),
        fill = null,
        stroke = INK,
        strokeLineWidth = STROKE_WIDTH,
        strokeLineCap = StrokeCap.Round,
        strokeLineJoin = StrokeJoin.Round,
    )

/** A full circle as two half arcs, so it stays one closed subpath. */
internal fun PathBuilder.circle(
    cx: Float,
    cy: Float,
    r: Float,
) {
    moveTo(cx - r, cy)
    arcTo(r, r, 0f, false, true, cx + r, cy)
    arcTo(r, r, 0f, false, true, cx - r, cy)
    close()
}
