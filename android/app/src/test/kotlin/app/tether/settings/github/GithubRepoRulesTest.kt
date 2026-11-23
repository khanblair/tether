package app.tether.settings.github

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GithubRepoRulesTest {
    private val repos =
        listOf(
            GithubRepo("https://github.com/Octo/Acme-API.git", "main", true, 100),
            GithubRepo("https://github.com/octo/tether.git", "main", false, 100),
            GithubRepo("https://github.com/kolab/mobile", "develop", true, 100),
        )

    @Test
    fun the_slug_comes_from_https_and_ssh_clone_urls() {
        assertEquals("octo/tether", repoSlug("https://github.com/octo/tether.git"))
        assertEquals("octo/tether", repoSlug("git@github.com:octo/tether.git"))
        assertEquals("octo/tether", repoSlug("  https://github.com/octo/tether/  "))
    }

    @Test
    fun a_url_without_an_owner_and_name_has_no_slug() {
        assertNull(repoSlug("https://github.com/octo"))
        assertNull(repoSlug("https://example.com/octo/tether.git"))
        assertNull(repoSlug(""))
    }

    @Test
    fun the_project_name_is_the_repository_name() {
        assertEquals("tether", defaultProjectName("https://github.com/octo/tether.git"))
        assertEquals("tether", defaultProjectName("https://example.com/team/tether.git"))
        assertEquals("", defaultProjectName(""))
    }

    @Test
    fun the_filter_matches_slug_or_url_ignoring_case() {
        assertEquals(listOf(repos[0]), filterRepos(repos, "acme"))
        assertEquals(listOf(repos[0], repos[1]), filterRepos(repos, "OCTO"))
        assertEquals(listOf(repos[2]), filterRepos(repos, "kolab/mob"))
    }

    @Test
    fun a_blank_filter_keeps_every_repository_in_order() {
        assertEquals(repos, filterRepos(repos, "   "))
        assertEquals(emptyList<GithubRepo>(), filterRepos(repos, "missing"))
    }

    @Test
    fun sizes_round_to_whole_megabytes() {
        assertEquals(0L, megabytesFromKb(511))
        assertEquals(1L, megabytesFromKb(512))
        assertEquals(1L, megabytesFromKb(1024))
        assertEquals(50L, megabytesFromKb(51_200))
    }

    @Test
    fun byte_counts_show_one_decimal_megabyte() {
        assertEquals("8.2", megabytesText(8_598_323))
        assertEquals("0.0", megabytesText(0))
        assertEquals("13.1", megabytesText(13_745_128))
    }
}
