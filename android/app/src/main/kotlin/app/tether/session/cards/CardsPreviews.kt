package app.tether.session.cards

import android.content.res.Configuration
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.PlanUi
import app.tether.session.agent.QuestionUi
import app.tether.session.agent.TodoStatus
import app.tether.session.agent.TodoUi

private val sampleTodos =
    listOf(
        TodoUi("Find the expiry check", TodoStatus.Completed),
        TodoUi("Fix the boundary comparison", TodoStatus.Completed),
        TodoUi("Run the login tests", TodoStatus.InProgress),
        TodoUi("Add a regression test", TodoStatus.Pending),
        TodoUi("Update the changelog", TodoStatus.Pending),
        TodoUi("Check the docs", TodoStatus.Pending),
        TodoUi("Open the pull request", TodoStatus.Pending),
    )

private val sampleQuestion =
    QuestionUi(
        toolCallId = "call-1",
        question = "The test expects a token to expire at its exact second. Which should change?",
        options = listOf("Fix the code to match the test", "Fix the test to match the docs"),
        allowFreeText = true,
    )

private val samplePlan =
    PlanUi(
        toolCallId = "call-2",
        summary = "Add a token bucket to the search route.",
        steps =
            listOf(
                "Add a token bucket in the limits module.",
                "Wrap the search handler.",
                "Return 429 with Retry-After.",
            ),
        files =
            listOf(
                "src/limits.py",
                "src/api/search/handler_with_a_long_name_for_display.py",
            ),
        commands = listOf("pytest -q"),
        openQuestions = listOf("Limit per API key or per IP?"),
    )

/** Lays a preview out in the theme it names, with room around the card. */
@Composable
private fun Themed(
    mode: ThemeMode,
    content: @Composable () -> Unit,
) {
    TetherTheme(mode = mode) {
        Column(modifier = Modifier.padding(16.dp)) { content() }
    }
}

@Preview(name = "Checklist collapsed", showBackground = true)
@Composable
internal fun ChecklistCollapsedPreview() =
    Themed(ThemeMode.Light) {
        ChecklistPin(sampleTodos, currentChecklistItem(sampleTodos), false, {}, {})
    }

@Preview(name = "Checklist expanded", showBackground = true)
@Composable
internal fun ChecklistExpandedPreview() =
    Themed(ThemeMode.Light) {
        ChecklistPin(sampleTodos, currentChecklistItem(sampleTodos), true, {}, {})
    }

@Preview(name = "Checklist all done", showBackground = true)
@Composable
internal fun ChecklistAllDonePreview() {
    val done = sampleTodos.map { it.copy(status = TodoStatus.Completed) }
    Themed(ThemeMode.Light) { ChecklistPin(done, currentChecklistItem(done), false, {}, {}) }
}

@Preview(name = "Checklist dark", uiMode = Configuration.UI_MODE_NIGHT_YES, showBackground = true)
@Composable
internal fun ChecklistDarkPreview() =
    Themed(ThemeMode.Dark) {
        ChecklistPin(sampleTodos, currentChecklistItem(sampleTodos), true, {}, {})
    }

@Preview(name = "Question open", showBackground = true)
@Composable
internal fun AskOpenPreview() =
    Themed(ThemeMode.Light) {
        AskUserCard(sampleQuestion, null, "", false, null, {}, {}, {})
    }

@Preview(name = "Question with draft", showBackground = true)
@Composable
internal fun AskDraftPreview() =
    Themed(ThemeMode.Light) {
        AskUserCard(sampleQuestion, null, "Keep the test as it is", false, null, {}, {}, {})
    }

@Preview(name = "Question failed", showBackground = true)
@Composable
internal fun AskFailedPreview() =
    Themed(ThemeMode.Light) {
        AskUserCard(sampleQuestion, null, "", false, AgentLoopFailure.Busy, {}, {}, {})
    }

@Preview(name = "Question answered", showBackground = true)
@Composable
internal fun AskAnsweredPreview() =
    Themed(ThemeMode.Light) {
        AskUserCard(sampleQuestion, "Fix the code to match the test", "", false, null, {}, {}, {})
    }

@Preview(name = "Question dark", uiMode = Configuration.UI_MODE_NIGHT_YES, showBackground = true)
@Composable
internal fun AskDarkPreview() =
    Themed(ThemeMode.Dark) {
        AskUserCard(sampleQuestion, null, "", false, null, {}, {}, {})
    }

@Preview(name = "Plan default", showBackground = true)
@Composable
internal fun PlanDefaultPreview() = PlanPreview(ThemeMode.Light, PlanPreviewState())

@Preview(name = "Plan editing", showBackground = true)
@Composable
internal fun PlanEditingPreview() = PlanPreview(ThemeMode.Light, PlanPreviewState(editing = true))

@Preview(name = "Plan run choice", showBackground = true)
@Composable
internal fun PlanRunChoicePreview() =
    PlanPreview(ThemeMode.Light, PlanPreviewState(runChoiceOpen = true))

@Preview(name = "Plan started", showBackground = true)
@Composable
internal fun PlanStartedPreview() = PlanPreview(ThemeMode.Light, PlanPreviewState(started = true))

@Preview(name = "Plan dark", uiMode = Configuration.UI_MODE_NIGHT_YES, showBackground = true)
@Composable
internal fun PlanDarkPreview() = PlanPreview(ThemeMode.Dark, PlanPreviewState())

/** The inputs a plan preview varies. */
private data class PlanPreviewState(
    val started: Boolean = false,
    val editing: Boolean = false,
    val runChoiceOpen: Boolean = false,
)

@Composable
private fun PlanPreview(
    mode: ThemeMode,
    state: PlanPreviewState,
) {
    Themed(mode) {
        PlanCard(
            plan = samplePlan,
            started = state.started,
            editing = state.editing,
            planDraft = "Add a token bucket to the search route.",
            runChoiceOpen = state.runChoiceOpen,
            collapsedSections = emptySet(),
            busy = false,
            failure = null,
            onRun = {},
            onChooseMode = {},
            onCancelRunChoice = {},
            onStartEdit = {},
            onPlanDraftChange = {},
            onCancelEdit = {},
            onEdit = {},
            onKeepPlanning = {},
            onToggleSection = {},
        )
    }
}
