package app.tether.settings.github

import org.junit.Assert.assertEquals
import org.junit.Test

class GithubModelsTest {
    @Test
    fun classic_access_keeps_scopes_and_reports_repository_reach() {
        val access =
            status(GithubTokenKind.Classic, canReadRepos = true, scopes = listOf("repo"))
                .accountAccess()

        assertEquals(GithubTokenKind.Classic, access.tokenKind)
        assertEquals(GithubRepositoryAccess.Yes, access.repositoryAccess)
        assertEquals(listOf("repo"), access.scopes)
        assertEquals(GithubScopePresentation.ClassicScopes, access.scopePresentation)
    }

    @Test
    fun classic_access_without_repository_reach_keeps_its_scopes() {
        val access =
            status(GithubTokenKind.Classic, canReadRepos = false, scopes = listOf("read:user"))
                .accountAccess()

        assertEquals(GithubRepositoryAccess.No, access.repositoryAccess)
        assertEquals(listOf("read:user"), access.scopes)
        assertEquals(GithubScopePresentation.ClassicScopes, access.scopePresentation)
    }

    @Test
    fun fine_grained_access_hides_scopes_and_reports_repository_reach() {
        val access = status(GithubTokenKind.FineGrained, canReadRepos = true).accountAccess()

        assertEquals(GithubRepositoryAccess.Yes, access.repositoryAccess)
        assertEquals(emptyList<String>(), access.scopes)
        assertEquals(GithubScopePresentation.FineGrained, access.scopePresentation)
    }

    @Test
    fun fine_grained_access_without_repository_reach_hides_scopes() {
        val access = status(GithubTokenKind.FineGrained, canReadRepos = false).accountAccess()

        assertEquals(GithubRepositoryAccess.No, access.repositoryAccess)
        assertEquals(emptyList<String>(), access.scopes)
        assertEquals(GithubScopePresentation.FineGrained, access.scopePresentation)
    }

    private fun status(
        tokenKind: GithubTokenKind,
        canReadRepos: Boolean,
        scopes: List<String> = emptyList(),
    ) = GithubStatus(
        signedIn = true,
        login = "octo",
        name = null,
        email = null,
        scopes = scopes,
        tokenKind = tokenKind,
        canReadRepos = canReadRepos,
    )
}
