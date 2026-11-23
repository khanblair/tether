package app.tether.git

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode

private val sampleDiff =
    listOf(
        "diff --git a/src/auth/session.py b/src/auth/session.py",
        "--- a/src/auth/session.py",
        "+++ b/src/auth/session.py",
        "@@ -41,7 +41,7 @@ is_expired",
        " def is_expired(token, now):",
        "-    return token.expires_at < now",
        "+    return token.expires_at <= now",
        " ",
        "diff --git a/assets/logo.png b/assets/logo.png",
        "Binary files a/assets/logo.png and b/assets/logo.png differ",
    ).joinToString("\n")

private val sampleFiles = UnifiedDiffParser.parse(sampleDiff)

@Preview(name = "Diff, all changes, wrapped, light")
@Composable
internal fun DiffWrappedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        DiffScreen(
            state =
                DiffUiState(
                    path = null,
                    staged = false,
                    content = Loadable.Ready(DiffContent(sampleFiles, false)),
                ),
            onSetStaged = {},
            onToggleWrap = {},
            onRetry = {},
            onBack = {},
        )
    }
}

@Preview(name = "Diff, one file, not wrapped, dark")
@Composable
internal fun DiffUnwrappedDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        DiffScreen(
            state =
                DiffUiState(
                    path = "src/auth/session.py",
                    staged = true,
                    wrap = false,
                    content = Loadable.Ready(DiffContent(sampleFiles.take(1), false)),
                ),
            onSetStaged = {},
            onToggleWrap = {},
            onRetry = {},
            onBack = {},
        )
    }
}

@Preview(name = "Diff, cut at the size limit, light")
@Composable
internal fun DiffTruncatedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        DiffScreen(
            state =
                DiffUiState(
                    path = null,
                    staged = false,
                    content = Loadable.Ready(DiffContent(sampleFiles, true)),
                ),
            onSetStaged = {},
            onToggleWrap = {},
            onRetry = {},
            onBack = {},
        )
    }
}

@Preview(name = "Diff, no changes in this file, light")
@Composable
internal fun DiffEmptyPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        DiffScreen(
            state =
                DiffUiState(
                    path = "notes/untracked-draft.md",
                    staged = false,
                    content = Loadable.Ready(DiffContent(emptyList(), false)),
                ),
            onSetStaged = {},
            onToggleWrap = {},
            onRetry = {},
            onBack = {},
        )
    }
}

@Preview(name = "Diff, failed, light")
@Composable
internal fun DiffFailedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        DiffScreen(
            state =
                DiffUiState(
                    path = null,
                    staged = false,
                    content = Loadable.Failed(GitFailure.Unavailable),
                ),
            onSetStaged = {},
            onToggleWrap = {},
            onRetry = {},
            onBack = {},
        )
    }
}
