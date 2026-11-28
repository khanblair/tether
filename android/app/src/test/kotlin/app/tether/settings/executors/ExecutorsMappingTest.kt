package app.tether.settings.executors

import app.tether.core.generated.CiRuntimeException
import app.tether.core.generated.ExecutorRuntimeException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class ExecutorsMappingTest {
    @Test
    fun every_executor_error_maps_to_one_failure() {
        assertEquals(
            ExecutorsFailure.NotConfigured,
            ExecutorRuntimeException.InvalidConfiguration().toFailure(),
        )
        assertEquals(
            ExecutorsFailure.Unavailable,
            ExecutorRuntimeException.Unavailable().toFailure(),
        )
        assertEquals(
            ExecutorsFailure.ProjectNotFound,
            ExecutorRuntimeException.ProjectNotFound().toFailure(),
        )
        assertEquals(
            ExecutorsFailure.VerificationFailed,
            ExecutorRuntimeException.VerificationFailed().toFailure(),
        )
        assertEquals(
            ExecutorsFailure.SetupFailed,
            ExecutorRuntimeException.SetupFailed("HTTP 503").toFailure(),
        )
    }

    @Test
    fun a_failed_setup_keeps_its_reason_and_other_errors_have_none() {
        assertEquals("HTTP 503", ExecutorRuntimeException.SetupFailed("HTTP 503").detail())
        assertEquals(null, ExecutorRuntimeException.Unavailable().detail())
        val thrown: Throwable = ExecutorsCallFailed(ExecutorsFailure.SetupFailed, "HTTP 503")
        assertEquals("HTTP 503", thrown.toExecutorsDetail())
        assertEquals(null, RuntimeException("x").toExecutorsDetail())
    }

    @Test
    fun the_linux_shell_errors_map_to_the_closest_failures() {
        assertEquals(
            ExecutorsFailure.Unavailable,
            ExecutorRuntimeException.LinuxNotReady().toFailure(),
        )
        assertEquals(
            ExecutorsFailure.Unknown,
            ExecutorRuntimeException.LinuxShellAlreadyOpen().toFailure(),
        )
    }

    @Test
    fun every_ci_error_maps_to_one_failure() {
        assertEquals(ExecutorsFailure.NoRepository, CiRuntimeException.Repository().toFailure())
        assertEquals(ExecutorsFailure.NotSignedIn, CiRuntimeException.NotSignedIn().toFailure())
        assertEquals(ExecutorsFailure.NoRemote, CiRuntimeException.NoRemote().toFailure())
        assertEquals(ExecutorsFailure.Unavailable, CiRuntimeException.Unavailable().toFailure())
        assertEquals(
            ExecutorsFailure.GithubError,
            CiRuntimeException.Github("token=secret").toFailure(),
        )
    }

    @Test
    fun the_github_detail_never_reaches_the_failure_message() {
        val failure = CiRuntimeException.Github("/Users/someone/key.pem").toFailure()

        assertEquals(ExecutorsFailure.GithubError.name, failure.name)
        assertFalse(ExecutorsCallFailed(failure).message.orEmpty().contains("/Users"))
    }

    @Test
    fun a_gateway_failure_keeps_its_failure_and_any_other_error_is_unknown() {
        assertEquals(
            ExecutorsFailure.VerificationFailed,
            ExecutorsCallFailed(ExecutorsFailure.VerificationFailed).toExecutorsFailure(),
        )
        assertEquals(ExecutorsFailure.Unknown, IllegalStateException("boom").toExecutorsFailure())
    }

    @Test
    fun ci_readiness_reads_the_core_reason_first() {
        assertEquals(
            CiReadiness.NotSignedIn,
            ciReadinessOf(
                ready = false,
                workflowPresent = false,
                workflowCurrent = false,
                reason = "GitHub is not signed in",
            ),
        )
        assertEquals(
            CiReadiness.NoRemote,
            ciReadinessOf(
                ready = false,
                workflowPresent = true,
                workflowCurrent = true,
                reason = "project does not have a GitHub remote",
            ),
        )
        assertEquals(
            CiReadiness.WorkflowMissing,
            ciReadinessOf(
                ready = true,
                workflowPresent = false,
                workflowCurrent = false,
                reason = "workflow is missing",
            ),
        )
        assertEquals(
            CiReadiness.WorkflowOutdated,
            ciReadinessOf(
                ready = true,
                workflowPresent = true,
                workflowCurrent = false,
                reason = "workflow is outdated",
            ),
        )
    }

    @Test
    fun ci_is_ready_only_when_signed_in_with_a_current_workflow() {
        assertEquals(
            CiReadiness.Ready,
            ciReadinessOf(
                ready = true,
                workflowPresent = true,
                workflowCurrent = true,
                reason = "",
            ),
        )
        assertEquals(
            CiReadiness.Unavailable,
            ciReadinessOf(
                ready = false,
                workflowPresent = false,
                workflowCurrent = false,
                reason = "project repository is unavailable",
            ),
        )
    }

    @Test
    fun run_states_come_from_status_and_conclusion() {
        assertEquals(CiRunState.Queued, ciRunStateOf("queued", ""))
        assertEquals(CiRunState.Running, ciRunStateOf("in_progress", ""))
        assertEquals(CiRunState.Succeeded, ciRunStateOf("completed", "success"))
        assertEquals(CiRunState.Failed, ciRunStateOf("completed", "failure"))
        assertEquals(CiRunState.Failed, ciRunStateOf("completed", "timed_out"))
        assertEquals(CiRunState.Cancelled, ciRunStateOf("completed", "cancelled"))
        assertEquals(CiRunState.Finished, ciRunStateOf("completed", "skipped"))
    }

    @Test
    fun an_unknown_run_status_never_claims_success_or_failure() {
        assertEquals(CiRunState.Finished, ciRunStateOf("weird", "success"))
        assertEquals(CiRunState.Finished, ciRunStateOf("", ""))
    }
}
