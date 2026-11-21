package app.tether.memory

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class SkillsViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val invalid =
        SkillItem(
            folder = "broken",
            name = "",
            description = "",
            valid = false,
            error = "Missing name",
        )

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun the_list_shows_every_skill_when_the_query_is_blank() =
        runTest(dispatcher) {
            val vm =
                SkillsViewModel(
                    FakeSkillsGateway(listOf(skill("fix-tests"), skill("release-notes"))),
                    dispatcher,
                )
            advanceUntilIdle()
            assertFalse(vm.state.value.loading)
            assertEquals(
                listOf("fix-tests", "release-notes"),
                vm.state.value.skills
                    .map { it.folder },
            )
            assertEquals(2, vm.state.value.total)
        }

    @Test
    fun the_query_matches_the_folder_name_or_the_description() =
        runTest(dispatcher) {
            val items =
                listOf(
                    skill("fix-tests"),
                    skill("release-notes", "Draft notes from merged pull requests"),
                )
            val vm = SkillsViewModel(FakeSkillsGateway(items), dispatcher)
            advanceUntilIdle()
            vm.onQueryChange("merged")
            assertEquals(
                listOf("release-notes"),
                vm.state.value.skills
                    .map { it.folder },
            )
            assertEquals(2, vm.state.value.total)
            vm.onQueryChange("FIX")
            assertEquals(
                listOf("fix-tests"),
                vm.state.value.skills
                    .map { it.folder },
            )
        }

    @Test
    fun an_invalid_skill_stays_in_the_list_with_its_error() =
        runTest(dispatcher) {
            val vm = SkillsViewModel(FakeSkillsGateway(listOf(invalid)), dispatcher)
            advanceUntilIdle()
            val row =
                vm.state.value.skills
                    .single()
            assertFalse(row.valid)
            assertEquals("Missing name", row.error)
        }

    @Test
    fun a_failed_list_shows_the_problem_and_no_skills() =
        runTest(dispatcher) {
            val gateway = FakeSkillsGateway(listOf(skill("fix-tests")))
            gateway.failure = AreaFailure(AreaProblem.Unavailable)
            val vm = SkillsViewModel(gateway, dispatcher)
            advanceUntilIdle()
            assertEquals(AreaProblem.Unavailable, vm.state.value.problem)
            assertEquals(emptyList<SkillItem>(), vm.state.value.skills)
            assertFalse(vm.state.value.loading)
        }

    @Test
    fun filtering_ignores_case_and_surrounding_spaces() {
        val items = listOf(skill("pdf-extract"), skill("fix-tests"))
        assertEquals(listOf("pdf-extract"), filterSkills(items, "  PDF ").map { it.folder })
        assertEquals(items, filterSkills(items, ""))
    }

    @Test
    fun the_detail_shows_the_parsed_skill() =
        runTest(dispatcher) {
            val detail =
                SkillDetail(
                    "fix-tests",
                    "fix-tests",
                    "Fix tests",
                    "MIT",
                    null,
                    "run git",
                    mapOf(
                        "tether-requires-tools" to "run",
                    ),
                    "# Fix",
                )
            val vm =
                SkillDetailViewModel(
                    FakeSkillsGateway(details = mapOf("fix-tests" to detail)),
                    "fix-tests",
                    dispatcher,
                )
            advanceUntilIdle()
            assertEquals(detail, vm.state.value.detail)
            assertNull(vm.state.value.problem)
        }

    @Test
    fun a_missing_skill_shows_not_found() =
        runTest(dispatcher) {
            val vm = SkillDetailViewModel(FakeSkillsGateway(), "gone", dispatcher)
            advanceUntilIdle()
            assertNull(vm.state.value.detail)
            assertEquals(AreaProblem.NotFound, vm.state.value.problem)
        }
}
