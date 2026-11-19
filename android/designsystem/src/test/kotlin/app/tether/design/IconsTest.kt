package app.tether.design

import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class IconsTest {
    private val icons: List<ImageVector> =
        listOf(
            TetherIcons.CircleCheck,
            TetherIcons.CircleX,
            TetherIcons.CircleDot,
            TetherIcons.ChevronRight,
            TetherIcons.ChevronLeft,
            TetherIcons.Zap,
            TetherIcons.Trash2,
            TetherIcons.Eye,
            TetherIcons.EyeOff,
            TetherIcons.KeyRound,
        )

    @Test
    fun everyIconUsesTheTwentyFourDpGrid() {
        icons.forEach { icon ->
            assertEquals(icon.name, 24.dp, icon.defaultWidth)
            assertEquals(icon.name, 24.dp, icon.defaultHeight)
            assertEquals(icon.name, 24f, icon.viewportWidth, 0f)
            assertEquals(icon.name, 24f, icon.viewportHeight, 0f)
        }
    }

    @Test
    fun everyIconHasDrawnContent() {
        icons.forEach { icon ->
            assertTrue(icon.name, icon.root.size > 0)
        }
    }
}
