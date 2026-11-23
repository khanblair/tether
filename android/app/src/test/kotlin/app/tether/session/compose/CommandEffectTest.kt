package app.tether.session.compose

import app.tether.session.compose.CommandEffect.ClearContext
import app.tether.session.compose.CommandEffect.ExportSession
import app.tether.session.compose.CommandEffect.NewSession
import app.tether.session.compose.CommandEffect.OpenModeSheet
import app.tether.session.compose.CommandEffect.OpenModelSheet
import app.tether.session.compose.CommandEffect.OpenRewindSheet
import app.tether.session.compose.CommandEffect.OpenScreen
import app.tether.session.compose.CommandEffect.SendPrompt
import app.tether.session.compose.CommandEffect.SetPlanMode
import app.tether.session.compose.CommandEffect.Unsupported
import org.junit.Assert.assertEquals
import org.junit.Test

class CommandEffectTest {
    private fun resolved(
        name: String,
        kind: String,
        arguments: String = "",
        prompt: String? = null,
    ) = CommandResolution.Resolved(name, arguments, kind, prompt)

    @Test fun newClearPlanAndExportAreCoreActions() {
        assertEquals(NewSession, commandEffectOf(resolved("new", "action")))
        assertEquals(ClearContext, commandEffectOf(resolved("clear", "action")))
        assertEquals(SetPlanMode, commandEffectOf(resolved("plan", "action")))
        assertEquals(ExportSession, commandEffectOf(resolved("export", "action")))
    }

    @Test fun modelModeAndRewindOpenTheirSheets() {
        assertEquals(OpenModelSheet, commandEffectOf(resolved("model", "open")))
        assertEquals(OpenModeSheet, commandEffectOf(resolved("mode", "open")))
        assertEquals(OpenRewindSheet, commandEffectOf(resolved("rewind", "open")))
    }

    @Test fun screenCommandsOpenTheirScreen() {
        assertEquals(OpenScreen(CommandScreen.Diff, ""), commandEffectOf(resolved("diff", "open")))
        assertEquals(
            OpenScreen(CommandScreen.Linux, ""),
            commandEffectOf(resolved("linux", "open")),
        )
        assertEquals(OpenScreen(CommandScreen.CiRuns, ""), commandEffectOf(resolved("ci", "open")))
        assertEquals(OpenScreen(CommandScreen.Cost, ""), commandEffectOf(resolved("cost", "open")))
        assertEquals(
            OpenScreen(CommandScreen.Context, ""),
            commandEffectOf(resolved("context", "open")),
        )
        assertEquals(
            OpenScreen(CommandScreen.Memory, ""),
            commandEffectOf(resolved("memory", "open")),
        )
        assertEquals(OpenScreen(CommandScreen.Help, ""), commandEffectOf(resolved("help", "open")))
    }

    @Test fun skillsAndVaultKeepTheirArguments() {
        assertEquals(
            OpenScreen(CommandScreen.Skills, "fix-tests"),
            commandEffectOf(resolved("skills", "open", arguments = "fix-tests")),
        )
        assertEquals(
            OpenScreen(CommandScreen.Vault, "lint"),
            commandEffectOf(resolved("vault", "open", arguments = "lint")),
        )
    }

    @Test fun promptCommandsSendTheFinishedPrompt() {
        val prompts =
            listOf(
                "commit" to
                    "Write a commit message for the current changes, then commit them. fix typo",
                "push" to "Push the current branch. origin",
                "pr" to "Draft and open a pull request for the current changes. ",
                "review" to "Review the uncommitted changes with a second model. auth",
                "init" to "Inspect the repository manifests. ",
            )
        for ((name, text) in prompts) {
            assertEquals(SendPrompt(text), commandEffectOf(resolved(name, "prompt", prompt = text)))
        }
    }

    @Test fun promptCommandWithoutPromptTextIsUnsupported() {
        assertEquals(
            Unsupported("commit"),
            commandEffectOf(resolved("commit", "prompt", prompt = null)),
        )
    }

    @Test fun runAndCompactAreUnsupported() {
        assertEquals(Unsupported("run"), commandEffectOf(resolved("run", "run", arguments = "ls")))
        assertEquals(Unsupported("compact"), commandEffectOf(resolved("compact", "compact")))
    }

    @Test fun unknownNameIsUnsupportedAndNeverAPrompt() {
        assertEquals(
            Unsupported("frobnicate"),
            commandEffectOf(resolved("frobnicate", "prompt", prompt = "Do it.")),
        )
    }

    @Test fun kindMismatchIsUnsupported() {
        assertEquals(Unsupported("new"), commandEffectOf(resolved("new", "open")))
        assertEquals(Unsupported("diff"), commandEffectOf(resolved("diff", "action")))
        assertEquals(Unsupported("commit"), commandEffectOf(resolved("commit", "action")))
        assertEquals(
            Unsupported("model"),
            commandEffectOf(resolved("model", "prompt", prompt = "x")),
        )
    }

    @Test fun everyBuiltInCommandMapsToItsEffect() {
        val rows =
            listOf(
                Row("new", "action", NewSession),
                Row("clear", "action", ClearContext),
                Row("plan", "action", SetPlanMode),
                Row("export", "action", ExportSession),
                Row("model", "open", OpenModelSheet),
                Row("mode", "open", OpenModeSheet),
                Row("rewind", "open", OpenRewindSheet),
                Row("diff", "open", OpenScreen(CommandScreen.Diff, "")),
                Row("linux", "open", OpenScreen(CommandScreen.Linux, "")),
                Row("ci", "open", OpenScreen(CommandScreen.CiRuns, "")),
                Row("cost", "open", OpenScreen(CommandScreen.Cost, "")),
                Row("context", "open", OpenScreen(CommandScreen.Context, "")),
                Row("skills", "open", OpenScreen(CommandScreen.Skills, "")),
                Row("memory", "open", OpenScreen(CommandScreen.Memory, "")),
                Row("vault", "open", OpenScreen(CommandScreen.Vault, "")),
                Row("help", "open", OpenScreen(CommandScreen.Help, "")),
                Row("commit", "prompt", SendPrompt("Commit prompt.")),
                Row("push", "prompt", SendPrompt("Push prompt.")),
                Row("pr", "prompt", SendPrompt("Pull request prompt.")),
                Row("review", "prompt", SendPrompt("Review prompt.")),
                Row("init", "prompt", SendPrompt("Init prompt.")),
            )
        assertEquals(21, rows.size)
        for (row in rows) {
            val prompt = (row.effect as? SendPrompt)?.text
            assertEquals(row.effect, commandEffectOf(resolved(row.name, row.kind, prompt = prompt)))
        }
    }

    private data class Row(
        val name: String,
        val kind: String,
        val effect: CommandEffect,
    )
}
