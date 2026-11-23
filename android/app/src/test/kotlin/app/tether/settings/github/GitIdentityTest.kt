package app.tether.settings.github

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class GitIdentityRulesTest {
    @Test
    fun identity_problems_follow_the_core_rules() {
        val cases =
            listOf(
                "" to IdentityProblem.Empty,
                "   " to IdentityProblem.Empty,
                "Ada <ada@example.com>" to IdentityProblem.Forbidden,
                "Ada > Lovelace" to IdentityProblem.Forbidden,
                "Ada\nLovelace" to IdentityProblem.Forbidden,
                "Ada\rLovelace" to IdentityProblem.Forbidden,
                "Ada Lovelace" to null,
                " ada@example.com " to null,
            )
        for ((value, expected) in cases) {
            assertEquals("value \"$value\"", expected, identityProblem(value))
        }
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
class GitIdentityViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeGithubGateway()

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun createViewModel(): GitIdentityViewModel {
        val vm = GitIdentityViewModel(gateway, io = dispatcher)
        dispatcher.scheduler.runCurrent()
        return vm
    }

    @Test
    fun a_signed_in_account_fills_the_empty_fields() {
        gateway.status = GithubStatus(true, "octo", "Octo Cat", "octo@example.com", listOf("repo"))
        val vm = createViewModel()
        assertEquals("Octo Cat", vm.state.value.name)
        assertEquals("octo@example.com", vm.state.value.email)
    }

    @Test
    fun a_signed_out_account_leaves_the_fields_empty() {
        val vm = createViewModel()
        assertEquals("", vm.state.value.name)
        assertEquals("", vm.state.value.email)
    }

    @Test
    fun save_refuses_an_invalid_field_without_a_call() {
        val vm = createViewModel()
        vm.onNameChange("Ada <x>")
        vm.onEmailChange("")
        vm.save()
        dispatcher.scheduler.runCurrent()
        assertEquals(IdentityProblem.Forbidden, vm.state.value.nameProblem)
        assertEquals(IdentityProblem.Empty, vm.state.value.emailProblem)
        assertTrue(gateway.identitiesSaved.isEmpty())
    }

    @Test
    fun save_stores_the_trimmed_identity_and_reports_saved() {
        val vm = createViewModel()
        vm.onNameChange("  Ada Lovelace ")
        vm.onEmailChange("ada@example.com ")
        vm.save()
        dispatcher.scheduler.runCurrent()
        assertEquals(listOf("Ada Lovelace" to "ada@example.com"), gateway.identitiesSaved)
        assertTrue(vm.state.value.saved)
        assertFalse(vm.state.value.saving)
    }

    @Test
    fun a_save_failure_shows_the_failure_and_is_not_saved() {
        gateway.identityFailure = GithubFailure.InvalidInput
        val vm = createViewModel()
        vm.onNameChange("Ada")
        vm.onEmailChange("ada@example.com")
        vm.save()
        dispatcher.scheduler.runCurrent()
        assertEquals(GithubFailure.InvalidInput, vm.state.value.failure)
        assertFalse(vm.state.value.saved)
        assertNull(vm.state.value.nameProblem)
    }
}
