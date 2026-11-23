@file:Suppress("MagicNumber") // Preview timestamps and sample sizes are not app constants.

package app.tether.git

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode

private val sampleChanges =
    ChangesContent(
        branch = "tether/fix-login-timeout",
        staged =
            listOf(
                ChangeEntry("src/auth/session.py", ChangeKind.Modified, null),
                ChangeEntry("CHANGELOG.md", ChangeKind.Added, null),
            ),
        unstaged =
            listOf(
                ChangeEntry("tests/test_login.py", ChangeKind.Modified, ChangeKind.Modified),
                ChangeEntry("notes/untracked-draft.md", null, ChangeKind.Untracked),
            ),
    )

private val sampleCommits =
    listOf(
        GitCommit(
            "9c1e4a2f7b10",
            "Fix token expiry at the boundary second",
            "Blair",
            1_699_996_400,
        ),
        GitCommit("3f2a9c1d0e4b", "Add login timeout setting", "Blair", 1_699_900_000),
        GitCommit("71be0d5a9c22", "Initial commit", "Blair", 1_690_000_000),
    )

private const val PREVIEW_NOW = 1_700_000_000_000L

@Preview(name = "Changes, light")
@Composable
internal fun ChangesLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GitScreen(
            state = GitUiState(changes = Loadable.Ready(sampleChanges)),
            onSelectTab = {},
            onRefresh = {},
            onOpenDiff = { _, _ -> },
            onBack = {},
        )
    }
}

@Preview(name = "Changes, dark")
@Composable
internal fun ChangesDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        GitScreen(
            state = GitUiState(changes = Loadable.Ready(sampleChanges)),
            onSelectTab = {},
            onRefresh = {},
            onOpenDiff = { _, _ -> },
            onBack = {},
        )
    }
}

@Preview(name = "Changes, no changes")
@Composable
internal fun ChangesEmptyPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GitScreen(
            state =
                GitUiState(
                    changes = Loadable.Ready(ChangesContent("main", emptyList(), emptyList())),
                ),
            onSelectTab = {},
            onRefresh = {},
            onOpenDiff = { _, _ -> },
            onBack = {},
        )
    }
}

@Preview(name = "Changes, not a repository")
@Composable
internal fun NotRepositoryPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GitScreen(
            state = GitUiState(changes = Loadable.Failed(GitFailure.NotRepository)),
            onSelectTab = {},
            onRefresh = {},
            onOpenDiff = { _, _ -> },
            onBack = {},
        )
    }
}

@Preview(name = "Changes, failed")
@Composable
internal fun ChangesFailedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GitScreen(
            state = GitUiState(changes = Loadable.Failed(GitFailure.RepositoryFailed)),
            onSelectTab = {},
            onRefresh = {},
            onOpenDiff = { _, _ -> },
            onBack = {},
        )
    }
}

@Preview(name = "Branches, unborn branch, light")
@Composable
internal fun BranchesPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GitScreen(
            state =
                GitUiState(
                    tab = GitTab.Branches,
                    branches =
                        Loadable.Ready(
                            BranchesContent(
                                current = "main",
                                branches =
                                    listOf(
                                        GitBranch("main", null),
                                        GitBranch("tether/fix-login-timeout", "3f2a9c1d0e4b"),
                                    ),
                            ),
                        ),
                ),
            onSelectTab = {},
            onRefresh = {},
            onOpenDiff = { _, _ -> },
            onBack = {},
        )
    }
}

@Preview(name = "History, light")
@Composable
internal fun HistoryPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GitScreen(
            state =
                GitUiState(
                    tab = GitTab.History,
                    history = Loadable.Ready(HistoryContent(sampleCommits, PREVIEW_NOW)),
                ),
            onSelectTab = {},
            onRefresh = {},
            onOpenDiff = { _, _ -> },
            onBack = {},
        )
    }
}
