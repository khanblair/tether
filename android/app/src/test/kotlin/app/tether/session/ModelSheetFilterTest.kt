package app.tether.session

import app.tether.core.generated.ProviderView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ModelSheetFilterTest {
    private val sonnet = model("claude-sonnet-5", "Sonnet 5.5", reasoning = true)
    private val haiku = model("claude-haiku-5", "Haiku 5.5")
    private val gpt5 = model("gpt-5", "GPT-5")
    private val gpt5Mini = model("gpt-5-mini", "GPT-5 mini")
    private val ling =
        model("inclusionai/ling-3.2-flash:free", "inclusionAI: Ling 3.2 Flash (free)")
    private val anthropic = ModelGroup(ProviderView.ANTHROPIC, listOf(sonnet, haiku))
    private val openai = ModelGroup(ProviderView.OPEN_AI, listOf(gpt5, gpt5Mini))
    private val openRouter = ModelGroup(ProviderView.OPEN_ROUTER, listOf(ling))
    private val groups = listOf(anthropic, openai, openRouter)

    private fun model(
        id: String,
        name: String,
        reasoning: Boolean? = null,
    ) = ModelChoice(id, name, null, null, null, null, null, reasoning)

    private fun ids(result: List<ModelGroup>): List<Pair<ProviderView, List<String>>> =
        result.map { group -> group.provider to group.models.map { it.id } }

    @Test
    fun an_empty_or_blank_query_keeps_every_group_and_model() {
        assertEquals(groups, filterModelGroups(groups, ""))
        assertEquals(groups, filterModelGroups(groups, "   "))
    }

    @Test
    fun a_word_matches_the_display_name_ignoring_case() {
        val result = filterModelGroups(groups, "SONNET")
        assertEquals(listOf(ProviderView.ANTHROPIC to listOf("claude-sonnet-5")), ids(result))
    }

    @Test
    fun a_word_matches_the_id_too() {
        val result = filterModelGroups(groups, "gpt-5-mini")
        assertEquals(listOf(ProviderView.OPEN_AI to listOf("gpt-5-mini")), ids(result))
    }

    @Test
    fun every_word_must_appear() {
        assertEquals(
            listOf(ProviderView.OPEN_AI to listOf("gpt-5-mini")),
            ids(filterModelGroups(groups, "gpt mini")),
        )
        assertTrue(filterModelGroups(groups, "sonnet haiku").isEmpty())
    }

    @Test
    fun words_are_split_on_spaces_and_trimmed() {
        val result = filterModelGroups(groups, "  ling   flash ")
        assertEquals(listOf(ProviderView.OPEN_ROUTER to listOf(ling.id)), ids(result))
    }

    @Test
    fun a_provider_without_a_match_is_dropped_and_order_is_kept() {
        val result = filterModelGroups(groups, "5")
        assertEquals(
            listOf(
                ProviderView.ANTHROPIC to listOf("claude-sonnet-5", "claude-haiku-5"),
                ProviderView.OPEN_AI to listOf("gpt-5", "gpt-5-mini"),
            ),
            ids(result),
        )
    }

    @Test
    fun no_match_gives_an_empty_list() {
        assertTrue(filterModelGroups(groups, "nothing like this").isEmpty())
    }

    @Test
    fun the_thinking_section_shows_with_an_empty_query_for_a_reasoning_model() {
        val state = sheet(current = ModelRef(ProviderView.ANTHROPIC, "claude-sonnet-5"))
        assertTrue(showsEffortSection(state, groups))
    }

    @Test
    fun the_thinking_section_hides_when_the_current_model_is_filtered_out() {
        val state =
            sheet(current = ModelRef(ProviderView.ANTHROPIC, "claude-sonnet-5"), query = "gpt")
        assertFalse(showsEffortSection(state, filterModelGroups(groups, "gpt")))
    }

    @Test
    fun the_thinking_section_stays_while_the_current_model_still_matches() {
        val state =
            sheet(current = ModelRef(ProviderView.ANTHROPIC, "claude-sonnet-5"), query = "sonnet")
        assertTrue(showsEffortSection(state, filterModelGroups(groups, "sonnet")))
    }

    @Test
    fun the_thinking_section_never_shows_for_a_model_that_does_not_reason() {
        val state = sheet(current = ModelRef(ProviderView.ANTHROPIC, "claude-haiku-5"))
        val withoutReasoning = state.copy(showsEffort = false)
        assertFalse(showsEffortSection(withoutReasoning, groups))
    }

    private fun sheet(
        current: ModelRef,
        query: String = "",
    ) = ModelSheetState(
        loading = false,
        groups = groups,
        current = current,
        showsEffort = true,
        query = query,
    )
}
