package app.tether.home

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ProjectsViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private val projects =
        listOf(
            ProjectSummary("1", "acme-api", "main", 3, lastActivityAt = 1_000L, running = true),
            ProjectSummary("2", "tether", "main", 0, lastActivityAt = 5_000L, running = false),
            ProjectSummary("3", "Blog-Site", "main", 0, lastActivityAt = 3_000L, running = false),
            ProjectSummary(
                "4",
                "dotfiles",
                "feat/zsh",
                1,
                lastActivityAt = 2_000L,
                running = false,
            ),
        )

    private class FakeProjects(
        var items: List<ProjectSummary>,
    ) : ProjectsSource {
        override fun projects(): List<ProjectSummary> = items
    }

    /** A view model whose first read has finished, so the list is ready to check. */
    private fun viewModel(items: List<ProjectSummary> = projects): ProjectsViewModel {
        val vm = ProjectsViewModel(FakeProjects(items), clock = { 42L }, io = dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        return vm
    }

    @Test
    fun the_list_is_loading_until_the_first_read_finishes() {
        val vm = ProjectsViewModel(FakeProjects(projects), clock = { 42L }, io = dispatcher)
        assertTrue(vm.state.value.loading)
        assertTrue(
            vm.state.value.projects
                .isEmpty(),
        )
        dispatcher.scheduler.advanceUntilIdle()
        assertFalse(vm.state.value.loading)
        assertEquals(4, vm.state.value.totalCount)
    }

    @Test
    fun refresh_reads_the_source_again_and_keeps_the_query() {
        val source = FakeProjects(projects)
        val vm = ProjectsViewModel(source, clock = { 42L }, io = dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        vm.onQueryChange("tether")
        source.items = projects + ProjectSummary("5", "tether-web", "main", 0, 9_000L, false)
        vm.refresh()
        dispatcher.scheduler.advanceUntilIdle()
        val state = vm.state.value
        assertEquals(5, state.totalCount)
        assertEquals(listOf("tether-web", "tether"), state.projects.map { it.name })
    }

    @Test
    fun the_list_starts_with_every_project_most_recent_first() {
        val names =
            viewModel()
                .state.value.projects
                .map { it.name }
        assertEquals(listOf("tether", "Blog-Site", "dotfiles", "acme-api"), names)
    }

    @Test
    fun the_query_matches_the_name_ignoring_case() {
        val vm = viewModel()
        vm.onQueryChange("BLOG")
        val state = vm.state.value
        assertEquals("BLOG", state.query)
        assertEquals(listOf("Blog-Site"), state.projects.map { it.name })
    }

    @Test
    fun the_query_matches_part_of_a_name() {
        val vm = viewModel()
        vm.onQueryChange("api")
        assertEquals(
            listOf("acme-api"),
            vm.state.value.projects
                .map { it.name },
        )
    }

    @Test
    fun spaces_around_the_query_are_ignored_but_the_query_is_kept_as_typed() {
        val vm = viewModel()
        vm.onQueryChange("  api ")
        val state = vm.state.value
        assertEquals("  api ", state.query)
        assertEquals(listOf("acme-api"), state.projects.map { it.name })
    }

    @Test
    fun a_query_with_no_match_gives_an_empty_list_and_keeps_the_total() {
        val vm = viewModel()
        vm.onQueryChange("zzz")
        val state = vm.state.value
        assertTrue(state.projects.isEmpty())
        assertEquals(4, state.totalCount)
    }

    @Test
    fun the_total_count_does_not_change_with_the_query() {
        val vm = viewModel()
        assertEquals(4, vm.state.value.totalCount)
        vm.onQueryChange("tether")
        assertEquals(4, vm.state.value.totalCount)
    }

    @Test
    fun an_empty_source_has_no_projects_and_a_zero_total() {
        val state = viewModel(emptyList()).state.value
        assertTrue(state.projects.isEmpty())
        assertEquals(0, state.totalCount)
    }

    @Test
    fun the_relative_times_use_the_clock() {
        assertEquals(42L, viewModel().state.value.now)
    }
}
