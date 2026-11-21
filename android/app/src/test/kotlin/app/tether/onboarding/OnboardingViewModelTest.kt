package app.tether.onboarding

import app.tether.settings.FakeSettingsStore
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class OnboardingViewModelTest {
    @Test
    fun the_flow_starts_on_welcome_with_the_saved_name() {
        val vm = OnboardingViewModel(FakeSettingsStore(name = "Blair"))

        assertEquals(OnboardingStep.Welcome, vm.state.value.step)
        assertEquals("Blair", vm.state.value.name)
        assertNull(vm.state.value.exit)
    }

    @Test
    fun next_walks_the_steps_in_order() {
        val vm = OnboardingViewModel(FakeSettingsStore())

        vm.next()
        assertEquals(OnboardingStep.AddKey, vm.state.value.step)
        vm.next()
        assertEquals(OnboardingStep.OpenProject, vm.state.value.step)
    }

    @Test
    fun next_on_the_last_step_stays_there() {
        val vm = OnboardingViewModel(FakeSettingsStore())
        vm.next()
        vm.next()

        vm.next()

        assertEquals(OnboardingStep.OpenProject, vm.state.value.step)
    }

    @Test
    fun back_walks_one_step_at_a_time_and_stops_at_welcome() {
        val vm = OnboardingViewModel(FakeSettingsStore())
        vm.next()
        vm.next()

        vm.back()
        assertEquals(OnboardingStep.AddKey, vm.state.value.step)
        vm.back()
        assertEquals(OnboardingStep.Welcome, vm.state.value.step)
        vm.back()
        assertEquals(OnboardingStep.Welcome, vm.state.value.step)
    }

    @Test
    fun leaving_welcome_saves_the_trimmed_name() {
        val settings = FakeSettingsStore()
        val vm = OnboardingViewModel(settings)
        vm.onNameChange("  Blair  ")

        vm.next()

        assertEquals("Blair", settings.name)
    }

    @Test
    fun skipping_saves_done_and_opens_home_without_saving_the_name() {
        val settings = FakeSettingsStore()
        val vm = OnboardingViewModel(settings)
        vm.onNameChange("Blair")

        vm.skip()

        assertTrue(settings.finished)
        assertEquals("", settings.name)
        assertEquals(OnboardingExit.Home, vm.state.value.exit)
    }

    @Test
    fun skipping_from_a_later_step_also_saves_done() {
        val settings = FakeSettingsStore()
        val vm = OnboardingViewModel(settings)
        vm.next()

        vm.skip()

        assertTrue(settings.finished)
        assertEquals(OnboardingExit.Home, vm.state.value.exit)
    }

    @Test
    fun choosing_a_new_project_saves_done_and_opens_the_new_project_screen() {
        val settings = FakeSettingsStore()
        val vm = OnboardingViewModel(settings)
        vm.next()
        vm.next()

        vm.openNewProject()

        assertTrue(settings.finished)
        assertEquals(OnboardingExit.NewProject, vm.state.value.exit)
    }

    @Test
    fun a_step_change_alone_does_not_finish_the_flow() {
        val settings = FakeSettingsStore()
        val vm = OnboardingViewModel(settings)

        vm.next()
        vm.back()

        assertFalse(settings.finished)
        assertNull(vm.state.value.exit)
    }
}
