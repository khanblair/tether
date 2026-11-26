package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class StatusLinePartsTest {
    @Test
    fun normalLineIsModelModeContextCost() {
        assertEquals(
            listOf(StatusPart.Model, StatusPart.Mode, StatusPart.Context, StatusPart.Cost),
            statusLineParts(bypass = false, hasContext = true, hasCost = true),
        )
    }

    @Test
    fun bypassLineStartsWithBypassAndDropsTheMode() {
        assertEquals(
            listOf(StatusPart.Bypass, StatusPart.Model, StatusPart.Context, StatusPart.Cost),
            statusLineParts(bypass = true, hasContext = true, hasCost = true),
        )
    }

    @Test
    fun unknownContextIsLeftOut() {
        assertEquals(
            listOf(StatusPart.Model, StatusPart.Mode, StatusPart.Cost),
            statusLineParts(bypass = false, hasContext = false, hasCost = true),
        )
    }

    @Test
    fun unknownCostIsLeftOutInBypass() {
        assertEquals(
            listOf(StatusPart.Bypass, StatusPart.Model, StatusPart.Context),
            statusLineParts(bypass = true, hasContext = true, hasCost = false),
        )
    }

    @Test
    fun modelDisplayNameIsUsedWhenTheModelIsListed() {
        val models = listOf(choice("claude-sonnet-5-5", "Sonnet 5.5"))
        assertEquals("Sonnet 5.5", modelDisplayName("claude-sonnet-5-5", models))
    }

    @Test
    fun modelIdIsUsedWhenTheModelIsNotListed() {
        assertEquals("claude-sonnet-5-5", modelDisplayName("claude-sonnet-5-5", emptyList()))
    }

    private fun choice(
        id: String,
        name: String,
    ) = ModelChoice(
        id = id,
        displayName = name,
        contextWindow = null,
        inputMicrosPerMillion = null,
        outputMicrosPerMillion = null,
        tools = null,
        images = null,
        reasoning = null,
    )
}
