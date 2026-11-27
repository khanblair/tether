package app.tether.settings

import app.tether.memory.FakeSkillsGateway
import app.tether.memory.SkillItem
import app.tether.memory.SkillsGateway
import app.tether.settings.budgets.BudgetLimits
import app.tether.settings.budgets.BudgetsGateway
import app.tether.settings.budgets.FakeBudgetsGateway
import app.tether.settings.github.FakeGithubGateway
import app.tether.settings.github.GithubGateway
import app.tether.settings.github.GithubStatus
import app.tether.settings.mcp.FakeMcpGateway
import app.tether.settings.mcp.McpGateway
import app.tether.settings.mcp.McpServer
import app.tether.settings.mcp.McpStatus
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class SettingsRowsViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val github = FakeGithubGateway()
    private val mcp = FakeMcpGateway()
    private val skills = FakeSkillsGateway()
    private val budgets = FakeBudgetsGateway()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun createViewModel(
        github: GithubGateway = this.github,
        mcp: McpGateway = this.mcp,
        skills: SkillsGateway = this.skills,
        budgets: BudgetsGateway = this.budgets,
    ): SettingsRowsViewModel {
        val vm = SettingsRowsViewModel(github, mcp, skills, budgets, dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        return vm
    }

    private fun server(id: String) =
        McpServer(
            id = id,
            name = id,
            url = "https://example.test/$id",
            enabled = true,
            status = McpStatus.Connected,
            headerNames = emptyList(),
            tools = emptyList(),
        )

    private fun skill(folder: String) = SkillItem(folder, folder, "", valid = true, error = null)

    @Test
    fun before_the_reads_finish_every_row_has_no_subtitle() {
        val vm = SettingsRowsViewModel(github, mcp, skills, budgets, dispatcher)

        assertEquals(SettingsRowsState(), vm.state.value)
    }

    @Test
    fun a_signed_in_account_shows_its_login() {
        github.status = GithubStatus(true, "blair", null, null, emptyList())

        assertEquals(RowText.SignedIn("blair"), createViewModel().state.value.github)
    }

    @Test
    fun a_signed_in_account_without_a_login_has_no_login_text() {
        github.status = GithubStatus(true, null, null, null, emptyList())

        assertEquals(RowText.SignedIn(null), createViewModel().state.value.github)
    }

    @Test
    fun no_saved_account_says_not_signed_in() {
        assertEquals(RowText.NotSignedIn, createViewModel().state.value.github)
    }

    @Test
    fun the_mcp_row_counts_zero_one_and_many_servers() {
        assertEquals(RowText.Servers(0), createViewModel().state.value.mcp)

        mcp.serverList += server("a")
        assertEquals(RowText.Servers(1), createViewModel().state.value.mcp)

        mcp.serverList += listOf(server("b"), server("c"))
        assertEquals(RowText.Servers(3), createViewModel().state.value.mcp)
    }

    @Test
    fun the_skills_row_counts_every_installed_folder() {
        assertEquals(RowText.Skills(0), createViewModel().state.value.skills)

        val many = FakeSkillsGateway(items = listOf(skill("a"), skill("b"), skill("c")))
        assertEquals(RowText.Skills(3), createViewModel(skills = many).state.value.skills)
    }

    @Test
    fun no_limits_say_no_limits_set() {
        assertEquals(RowText.NoLimits, createViewModel().state.value.budgets)
    }

    @Test
    fun a_daily_limit_is_shown_and_wins_over_a_session_limit() {
        budgets.limits = BudgetLimits(dailyMicros = 20_000_000L, sessionMicros = 5_000_000L)

        assertEquals(RowText.DailyLimit(20_000_000L), createViewModel().state.value.budgets)
    }

    @Test
    fun a_session_limit_alone_is_shown() {
        budgets.limits = BudgetLimits(dailyMicros = null, sessionMicros = 5_000_000L)

        assertEquals(RowText.SessionLimit(5_000_000L), createViewModel().state.value.budgets)
    }

    @Test
    fun a_failed_github_read_leaves_no_subtitle_and_other_rows_still_load() {
        val broken =
            object : GithubGateway by github {
                override fun status(): GithubStatus = error("core down")
            }
        mcp.serverList += server("a")

        val state = createViewModel(github = broken).state.value

        assertEquals(RowText.None, state.github)
        assertEquals(RowText.Servers(1), state.mcp)
    }

    @Test
    fun a_failed_mcp_read_leaves_no_subtitle() {
        val broken =
            object : McpGateway by mcp {
                override fun servers(): List<McpServer> = error("core down")
            }

        assertEquals(RowText.None, createViewModel(mcp = broken).state.value.mcp)
    }

    @Test
    fun a_failed_skills_read_leaves_no_subtitle() {
        val broken =
            object : SkillsGateway by skills {
                override fun list(): List<SkillItem> = error("core down")
            }

        assertEquals(RowText.None, createViewModel(skills = broken).state.value.skills)
    }

    @Test
    fun a_failed_budgets_read_leaves_no_subtitle() {
        val broken =
            object : BudgetsGateway by budgets {
                override fun budgets(): BudgetLimits = error("core down")
            }

        assertEquals(RowText.None, createViewModel(budgets = broken).state.value.budgets)
    }

    @Test
    fun a_refresh_shows_the_new_count() {
        val vm = createViewModel()
        mcp.serverList += listOf(server("a"), server("b"))

        vm.refresh()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(RowText.Servers(2), vm.state.value.mcp)
    }
}
