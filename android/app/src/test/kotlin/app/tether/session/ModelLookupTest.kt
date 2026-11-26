package app.tether.session

import app.tether.core.generated.ProviderView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ModelLookupTest {
    private fun model(id: String) =
        ModelChoice(
            id = id,
            displayName = "Name of $id",
            contextWindow = null,
            inputMicrosPerMillion = null,
            outputMicrosPerMillion = null,
            tools = true,
            images = true,
            reasoning = true,
        )

    @Test
    fun the_list_is_read_once_for_the_same_provider_and_model() {
        val lookup = ModelLookup()
        var reads = 0
        val load = {
            reads++
            listOf(model("a"), model("b"))
        }
        repeat(5) { lookup.find(ProviderView.ANTHROPIC, "b", load) }
        assertEquals(1, reads)
        assertEquals("b", lookup.find(ProviderView.ANTHROPIC, "b", load)?.id)
    }

    @Test
    fun a_different_model_reads_the_list_again() {
        val lookup = ModelLookup()
        var reads = 0
        val load = {
            reads++
            listOf(model("a"), model("b"))
        }
        lookup.find(ProviderView.ANTHROPIC, "a", load)
        lookup.find(ProviderView.ANTHROPIC, "b", load)
        lookup.find(ProviderView.OPEN_AI, "b", load)
        assertEquals(3, reads)
    }

    @Test
    fun a_failed_read_is_not_remembered() {
        val lookup = ModelLookup()
        var fail = true
        val load = {
            if (fail) error("catalog unavailable")
            listOf(model("a"))
        }
        assertNull(lookup.find(ProviderView.ANTHROPIC, "a", load))
        fail = false
        assertEquals("a", lookup.find(ProviderView.ANTHROPIC, "a", load)?.id)
    }

    @Test
    fun a_model_that_is_not_in_the_list_gives_null() {
        val lookup = ModelLookup()
        assertNull(lookup.find(ProviderView.ANTHROPIC, "missing") { listOf(model("a")) })
    }
}
