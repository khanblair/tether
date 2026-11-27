@file:Suppress("MagicNumber") // Preview sample sizes and timings are not app constants.

package app.tether.settings.github

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode

private val sampleGrant =
    DeviceGrant(
        userCode = "ABCD-1234",
        verificationUri = "https://github.com/login/device",
        intervalSeconds = 5,
        expiresInSeconds = 900,
    )

private val sampleRepos =
    listOf(
        GithubRepo(
            "https://github.com/example/app-one.git",
            "main",
            isPrivate = true,
            sizeKb = 51_200,
        ),
        GithubRepo(
            "https://github.com/example/notes.git",
            "main",
            isPrivate = false,
            sizeKb = 2_048,
        ),
    )

@Preview(name = "Account signed out, light")
@Composable
internal fun AccountSignedOutPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GithubAccountScreen(
            state = AccountUiState(loading = false),
            onBack = {},
            onSignInWithDevice = {},
            onStopPolling = {},
            onOpenUrl = {},
            onTokenChange = {},
            onSignInWithToken = {},
            onSignOut = {},
            onOpenIdentity = {},
        )
    }
}

@Preview(name = "Account device code, dark")
@Composable
internal fun AccountDeviceCodePreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        GithubAccountScreen(
            state =
                AccountUiState(
                    loading = false,
                    device =
                        DeviceStep.Waiting(
                            sampleGrant,
                            System.currentTimeMillis() + sampleGrant.expiresInSeconds * 1000,
                        ),
                ),
            onBack = {},
            onSignInWithDevice = {},
            onStopPolling = {},
            onOpenUrl = {},
            onTokenChange = {},
            onSignInWithToken = {},
            onSignOut = {},
            onOpenIdentity = {},
        )
    }
}

@Preview(name = "Account classic token, repositories readable")
@Composable
internal fun AccountSignedInPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GithubAccountScreen(
            state =
                AccountUiState(
                    loading = false,
                    status =
                        GithubStatus(
                            signedIn = true,
                            login = "example-user",
                            name = "Example User",
                            email = "user@example.com",
                            scopes = listOf("repo"),
                            tokenKind = GithubTokenKind.Classic,
                            canReadRepos = true,
                        ),
                ),
            onBack = {},
            onSignInWithDevice = {},
            onStopPolling = {},
            onOpenUrl = {},
            onTokenChange = {},
            onSignInWithToken = {},
            onSignOut = {},
            onOpenIdentity = {},
        )
    }
}

@Preview(name = "Account classic token, repositories blocked")
@Composable
internal fun AccountClassicNoRepositoryAccessPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GithubAccountScreen(
            state =
                AccountUiState(
                    loading = false,
                    status =
                        GithubStatus(
                            signedIn = true,
                            login = "example-user",
                            name = "Example User",
                            email = "user@example.com",
                            scopes = listOf("read:user"),
                            tokenKind = GithubTokenKind.Classic,
                            canReadRepos = false,
                        ),
                ),
            onBack = {},
            onSignInWithDevice = {},
            onStopPolling = {},
            onOpenUrl = {},
            onTokenChange = {},
            onSignInWithToken = {},
            onSignOut = {},
            onOpenIdentity = {},
        )
    }
}

@Preview(name = "Account fine-grained token, repositories readable")
@Composable
internal fun AccountFineGrainedPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        GithubAccountScreen(
            state =
                AccountUiState(
                    loading = false,
                    status =
                        GithubStatus(
                            signedIn = true,
                            login = "example-user",
                            name = "Example User",
                            email = "user@example.com",
                            scopes = emptyList(),
                            tokenKind = GithubTokenKind.FineGrained,
                            canReadRepos = true,
                        ),
                ),
            onBack = {},
            onSignInWithDevice = {},
            onStopPolling = {},
            onOpenUrl = {},
            onTokenChange = {},
            onSignInWithToken = {},
            onSignOut = {},
            onOpenIdentity = {},
        )
    }
}

@Preview(name = "Account fine-grained token, repositories blocked")
@Composable
internal fun AccountFineGrainedNoRepositoryAccessPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GithubAccountScreen(
            state =
                AccountUiState(
                    loading = false,
                    status =
                        GithubStatus(
                            signedIn = true,
                            login = "example-user",
                            name = "Example User",
                            email = "user@example.com",
                            scopes = emptyList(),
                            tokenKind = GithubTokenKind.FineGrained,
                            canReadRepos = false,
                        ),
                ),
            onBack = {},
            onSignInWithDevice = {},
            onStopPolling = {},
            onOpenUrl = {},
            onTokenChange = {},
            onSignInWithToken = {},
            onSignOut = {},
            onOpenIdentity = {},
        )
    }
}

@Preview(name = "Account token refused, light")
@Composable
internal fun AccountTokenMissingScopePreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GithubAccountScreen(
            state =
                AccountUiState(
                    loading = false,
                    deviceAvailable = false,
                    failure = GithubFailure.MissingScope,
                ),
            onBack = {},
            onSignInWithDevice = {},
            onStopPolling = {},
            onOpenUrl = {},
            onTokenChange = {},
            onSignInWithToken = {},
            onSignOut = {},
            onOpenIdentity = {},
        )
    }
}

@Preview(name = "Git identity, light")
@Composable
internal fun IdentityPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        GitIdentityScreen(
            state = IdentityUiState(name = "Ada <x>", emailProblem = IdentityProblem.Empty),
            onBack = {},
            onNameChange = {},
            onEmailChange = {},
            onSave = {},
            onSaved = {},
        )
    }
}

@Preview(name = "Clone signed out, light")
@Composable
internal fun ClonePreview() {
    TetherTheme(mode = ThemeMode.Light) {
        CloneScreen(
            state = CloneUiState(signedIn = false),
            onBack = {},
            onUrlChange = {},
            onProjectNameChange = {},
            onHistoryChange = {},
            onQueryChange = {},
            onPickRepo = {},
            onLoadMore = {},
            onSignIn = {},
            onClone = {},
            onCancel = {},
            onCloned = {},
        )
    }
}

@Preview(name = "Clone repository list, dark")
@Composable
internal fun CloneListPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        CloneScreen(
            state =
                CloneUiState(
                    url = sampleRepos.first().cloneUrl,
                    projectName = "app-one",
                    signedIn = true,
                    repos = sampleRepos,
                    reposHaveMore = true,
                ),
            onBack = {},
            onUrlChange = {},
            onProjectNameChange = {},
            onHistoryChange = {},
            onQueryChange = {},
            onPickRepo = {},
            onLoadMore = {},
            onSignIn = {},
            onClone = {},
            onCancel = {},
            onCloned = {},
        )
    }
}

@Preview(name = "Clone in progress, light")
@Composable
internal fun CloneProgressPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        CloneScreen(
            state =
                CloneUiState(
                    url = sampleRepos.first().cloneUrl,
                    signedIn = true,
                    cloning = true,
                    progress = CloneProgress(ClonePhaseStep.Receiving, 8_598_323, 13_745_128),
                ),
            onBack = {},
            onUrlChange = {},
            onProjectNameChange = {},
            onHistoryChange = {},
            onQueryChange = {},
            onPickRepo = {},
            onLoadMore = {},
            onSignIn = {},
            onClone = {},
            onCancel = {},
            onCloned = {},
        )
    }
}

@Preview(name = "Clone failed, light")
@Composable
internal fun CloneFailedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        CloneScreen(
            state =
                CloneUiState(
                    url = sampleRepos.first().cloneUrl,
                    signedIn = true,
                    failure = GithubFailure.RepositoryFailed,
                ),
            onBack = {},
            onUrlChange = {},
            onProjectNameChange = {},
            onHistoryChange = {},
            onQueryChange = {},
            onPickRepo = {},
            onLoadMore = {},
            onSignIn = {},
            onClone = {},
            onCancel = {},
            onCloned = {},
        )
    }
}
