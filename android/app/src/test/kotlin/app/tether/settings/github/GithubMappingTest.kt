package app.tether.settings.github

import app.tether.core.generated.GitRuntimeException
import app.tether.core.generated.GithubStatusView
import app.tether.core.generated.GithubTokenKindView
import org.junit.Assert.assertEquals
import org.junit.Test

class GithubMappingTest {
    @Test
    fun github_status_maps_token_kind_repository_reach_and_scopes() {
        val status =
            GithubStatusView(
                signedIn = true,
                login = "octo",
                name = "Octo Cat",
                email = "octo@example.com",
                scopes = listOf("read:user"),
                tokenKind = GithubTokenKindView.FINE_GRAINED,
                canReadRepos = true,
            ).toGithubStatus()

        assertEquals(GithubTokenKind.FineGrained, status.tokenKind)
        assertEquals(true, status.canReadRepos)
        assertEquals(listOf("read:user"), status.scopes)
    }

    @Test
    fun every_core_error_maps_to_its_own_failure() {
        val cases =
            listOf(
                GitRuntimeException.NotFound() to GithubFailure.NotFound,
                GitRuntimeException.InvalidInput() to GithubFailure.InvalidInput,
                GitRuntimeException.Unavailable() to GithubFailure.Unavailable,
                GitRuntimeException.GithubNotConfigured() to GithubFailure.NotConfigured,
                GitRuntimeException.NotSignedIn() to GithubFailure.NotSignedIn,
                GitRuntimeException.NoDeviceGrant() to GithubFailure.NoDeviceGrant,
                GitRuntimeException.GithubCredentials() to GithubFailure.Rejected,
                GitRuntimeException.GithubResponse() to GithubFailure.BadResponse,
                GitRuntimeException.GithubNetwork() to GithubFailure.Network,
                GitRuntimeException.MissingScope("repo") to GithubFailure.MissingScope,
                GitRuntimeException.Repository("clone") to GithubFailure.RepositoryFailed,
            )
        for ((error, expected) in cases) {
            assertEquals(expected, error.toGithubFailure())
            assertEquals(expected, error.githubFailure())
        }
    }

    @Test
    fun any_other_error_counts_as_unavailable() {
        assertEquals(GithubFailure.Unavailable, IllegalStateException("x").githubFailure())
    }

    @Test
    fun a_gateway_failure_keeps_its_own_kind() {
        assertEquals(
            GithubFailure.Network,
            GithubFailureException(GithubFailure.Network).githubFailure(),
        )
    }

    @Test
    fun phase_names_map_as_the_core_names_them() {
        assertEquals(ClonePhaseStep.Resolving, phaseStepOf("Resolving"))
        assertEquals(ClonePhaseStep.Receiving, phaseStepOf("Receiving"))
        assertEquals(ClonePhaseStep.CheckingOut, phaseStepOf("CheckingOut"))
        assertEquals(ClonePhaseStep.Unknown, phaseStepOf("Something else"))
    }
}
