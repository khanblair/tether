package app.tether.design

import org.junit.Assert.assertEquals
import org.junit.Test

class ThemeModesTest {
    @Test
    fun systemGoesToLight() {
        assertEquals(ThemeMode.Light, ThemeMode.System.next())
    }

    @Test
    fun lightGoesToDark() {
        assertEquals(ThemeMode.Dark, ThemeMode.Light.next())
    }

    @Test
    fun darkGoesBackToSystem() {
        assertEquals(ThemeMode.System, ThemeMode.Dark.next())
    }
}
