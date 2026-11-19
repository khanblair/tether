package app.tether.theme

import app.tether.design.ThemeMode
import org.junit.Assert.assertEquals
import org.junit.Test

class ThemeControllerTest {
    private class FakeThemeStore(
        var saved: ThemeMode = ThemeMode.System,
    ) : ThemeStore {
        val saves = mutableListOf<ThemeMode>()

        override fun load(): ThemeMode = saved

        override fun save(mode: ThemeMode) {
            saves += mode
            saved = mode
        }
    }

    @Test
    fun the_controller_starts_from_the_stored_choice() {
        val controller = ThemeController(FakeThemeStore(ThemeMode.Dark))
        assertEquals(ThemeMode.Dark, controller.mode.value)
    }

    @Test
    fun set_updates_the_flow_and_saves_the_choice() {
        val store = FakeThemeStore()
        val controller = ThemeController(store)
        controller.set(ThemeMode.Light)
        assertEquals(ThemeMode.Light, controller.mode.value)
        assertEquals(listOf(ThemeMode.Light), store.saves)
    }

    @Test
    fun cycle_goes_system_light_dark_then_system_again() {
        val store = FakeThemeStore()
        val controller = ThemeController(store)
        controller.cycle()
        controller.cycle()
        controller.cycle()
        controller.cycle()
        assertEquals(
            listOf(ThemeMode.Light, ThemeMode.Dark, ThemeMode.System, ThemeMode.Light),
            store.saves,
        )
        assertEquals(ThemeMode.Light, controller.mode.value)
    }
}
