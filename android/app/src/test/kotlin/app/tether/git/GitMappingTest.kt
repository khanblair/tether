package app.tether.git

import app.tether.core.generated.GitRuntimeException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GitMappingTest {
    @Test
    fun every_core_error_maps_to_one_screen_failure() {
        assertEquals(GitFailure.NotFound, GitRuntimeException.NotFound().toGitFailure())
        assertEquals(GitFailure.InvalidInput, GitRuntimeException.InvalidInput().toGitFailure())
        assertEquals(GitFailure.Unavailable, GitRuntimeException.Unavailable().toGitFailure())
    }

    @Test
    fun the_account_variants_cannot_come_from_git_but_still_map() {
        assertEquals(
            GitFailure.Unavailable,
            GitRuntimeException.GithubNotConfigured().toGitFailure(),
        )
        assertEquals(GitFailure.Unavailable, GitRuntimeException.NotSignedIn().toGitFailure())
        assertEquals(GitFailure.Unavailable, GitRuntimeException.NoDeviceGrant().toGitFailure())
        assertEquals(GitFailure.Unavailable, GitRuntimeException.GithubCredentials().toGitFailure())
        assertEquals(GitFailure.Unavailable, GitRuntimeException.GithubResponse().toGitFailure())
        assertEquals(GitFailure.Unavailable, GitRuntimeException.GithubNetwork().toGitFailure())
        assertEquals(
            GitFailure.Unavailable,
            GitRuntimeException.MissingScope("repo").toGitFailure(),
        )
    }

    @Test
    fun a_repository_error_says_not_a_repository_only_for_that_text() {
        assertEquals(
            GitFailure.NotRepository,
            GitRuntimeException
                .Repository(
                    "Git repository operation failed: path is not a git repository",
                ).toGitFailure(),
        )
        assertEquals(
            GitFailure.RepositoryFailed,
            GitRuntimeException
                .Repository(
                    "Git repository operation failed: something else",
                ).toGitFailure(),
        )
    }

    @Test
    fun an_unborn_head_is_recognised_on_the_failure_cause() {
        val unborn = GitRuntimeException.Repository(UNBORN_HEAD_DETAIL)
        val failure = GitFailureException(GitFailure.RepositoryFailed, unborn)

        assertTrue(failure.isUnbornHead())
        assertFalse(GitFailureException(GitFailure.RepositoryFailed).isUnbornHead())
    }

    @Test
    fun any_other_error_counts_as_unavailable() {
        assertEquals(GitFailure.Unavailable, IllegalStateException("boom").gitFailure())
        assertEquals(GitFailure.NotFound, GitFailureException(GitFailure.NotFound).gitFailure())
    }

    @Test
    fun core_state_names_map_to_change_kinds() {
        assertEquals(ChangeKind.Added, changeKindOf("Added"))
        assertEquals(ChangeKind.Modified, changeKindOf("Modified"))
        assertEquals(ChangeKind.Deleted, changeKindOf("Deleted"))
        assertEquals(ChangeKind.Renamed, changeKindOf("Renamed"))
        assertEquals(ChangeKind.Untracked, changeKindOf("Untracked"))
        assertEquals(ChangeKind.Conflicted, changeKindOf("Conflicted"))
    }

    @Test
    fun an_empty_name_is_no_change_and_an_unknown_name_is_changed() {
        assertNull(changeKindOf(""))
        assertEquals(ChangeKind.Changed, changeKindOf("Copied"))
        assertEquals(ChangeKind.Changed, changeKindOf("Changed"))
    }
}
