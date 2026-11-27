package app.tether.appearance

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AppearanceControllerTest {
    private class FakeAppearanceStore(
        var saved: Appearance = Appearance(),
    ) : AppearanceStore {
        val saves = mutableListOf<Appearance>()

        override fun load(): Appearance = saved

        override fun save(appearance: Appearance) {
            saves += appearance
            saved = appearance
        }
    }

    @Test
    fun the_controller_starts_from_the_stored_values() {
        val controller =
            AppearanceController(FakeAppearanceStore(Appearance(125, 150, 0xFF001122.toInt())))
        assertEquals(Appearance(125, 150, 0xFF001122.toInt()), controller.appearance.value)
    }

    @Test
    fun the_controller_starts_from_normalized_stored_values() {
        val controller =
            AppearanceController(FakeAppearanceStore(Appearance(interfacePercent = 87)))
        assertEquals(90, controller.appearance.value.interfacePercent)
    }

    @Test
    fun each_setter_updates_the_flow_and_saves_at_once() {
        val store = FakeAppearanceStore()
        val controller = AppearanceController(store)

        controller.setInterfacePercent(110)
        assertEquals(110, controller.appearance.value.interfacePercent)
        assertEquals(110, store.saves.last().interfacePercent)

        controller.setTextPercent(115)
        assertEquals(115, controller.appearance.value.textPercent)
        assertEquals(115, store.saves.last().textPercent)

        controller.setAccent(0xFF336699.toInt())
        assertEquals(0xFF336699.toInt(), controller.appearance.value.accentArgb)
        assertEquals(0xFF336699.toInt(), store.saves.last().accentArgb)

        controller.setAccent(null)
        assertNull(store.saves.last().accentArgb)
        assertEquals(4, store.saves.size)
    }

    @Test
    fun a_setter_snaps_unknown_values_before_saving() {
        val store = FakeAppearanceStore()
        val controller = AppearanceController(store)
        controller.setInterfacePercent(87)
        controller.setTextPercent(77)
        assertEquals(
            Appearance(interfacePercent = 90, textPercent = TEXT_FOLLOW_SYSTEM),
            store.saves.last(),
        )
    }

    @Test
    fun reset_restores_the_defaults_and_saves_them() {
        val store = FakeAppearanceStore()
        val controller = AppearanceController(store)
        controller.setInterfacePercent(80)
        controller.setTextPercent(150)
        controller.setAccent(0xFF000000.toInt())

        controller.reset()

        assertEquals(Appearance(), controller.appearance.value)
        assertEquals(Appearance(), store.saves.last())
        assertEquals(4, store.saves.size)
    }
}
