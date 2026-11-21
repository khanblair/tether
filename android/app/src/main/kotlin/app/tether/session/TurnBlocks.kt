package app.tether.session

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.core.generated.CoreErrorView
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.SessionState
import app.tether.design.components.StatusMark
import app.tether.design.tokens.TetherSpace
import kotlinx.coroutines.delay

private const val THINKING_TICK_MS = 1_000L

/**
 * The blocks a turn adds below the messages: the failure, the stop, a failed send, and the
 * thinking label. Retry is null while a turn runs, so it cannot resend over a live turn.
 */
@Composable
fun TurnFooter(
    turn: TurnState,
    providerName: String,
    onRetry: (() -> Unit)?,
    onOpenProviders: (() -> Unit)?,
    onDismissStopped: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val retry = if (turn.running) null else onRetry
    Column(
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        modifier = modifier.padding(horizontal = TetherSpace.space4),
    ) {
        turn.failure?.let { error ->
            TurnProblemBlock(error, providerName, retry, onOpenProviders)
        }
        turn.stopped?.let { stopped -> StoppedBlock(stopped, onDismissStopped) }
        if (turn.sendFailed) {
            InlineBanner(
                text = stringResource(R.string.transcript_send_failed),
                tone = BannerTone.Danger,
            )
        }
        turn.requestingSinceMs?.let { startedAtMs -> ThinkingLabel(startedAtMs) }
    }
}

@Composable
private fun TurnProblemBlock(
    error: CoreErrorView,
    providerName: String,
    onRetry: (() -> Unit)?,
    onOpenProviders: (() -> Unit)?,
) {
    val problem = turnProblemOf(error)
    val retry = onRetry.takeIf { problem.offersRetry(error) }
    when (problem) {
        TurnProblem.KeyRejected ->
            InlineBanner(
                text = stringResource(R.string.states_key_rejected_body),
                tone = BannerTone.Danger,
                title = stringResource(R.string.states_key_rejected_title, providerName),
                actionLabel = stringResource(R.string.states_open_providers),
                onAction = onOpenProviders,
            )
        TurnProblem.RateLimited ->
            InlineBanner(
                text = stringResource(R.string.states_rate_body),
                tone = BannerTone.Warning,
                title = stringResource(R.string.states_rate_title, providerName),
                actionLabel = stringResource(R.string.states_retry_now),
                onAction = retry,
            )
        TurnProblem.Network ->
            InlineBanner(
                text = stringResource(R.string.states_network_body),
                tone = BannerTone.Danger,
                title = stringResource(R.string.states_network_title),
                actionLabel = stringResource(R.string.states_retry),
                onAction = retry,
            )
        TurnProblem.Other ->
            InlineBanner(
                text = error.message.ifBlank { stringResource(R.string.transcript_turn_failed) },
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.states_retry),
                onAction = retry,
            )
    }
}

@Composable
private fun StoppedBlock(
    stopped: StoppedTurn,
    onDismiss: () -> Unit,
) {
    val tool = stopped.proposedTool
    InlineBanner(
        text =
            if (tool == null) {
                stringResource(R.string.states_stopped_body)
            } else {
                stringResource(R.string.states_stopped_running, tool.first, tool.second)
            },
        tone = BannerTone.Info,
        title = stringResource(R.string.states_stopped_title),
        actionLabel = stringResource(R.string.states_dismiss),
        onAction = onDismiss,
    )
}

/** "Thinking" while the core waits for the first token. Seconds show after the delay. */
@Composable
private fun ThinkingLabel(startedAtMs: Long) {
    var nowMs by remember(startedAtMs) { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(startedAtMs) {
        while (true) {
            delay(THINKING_TICK_MS)
            nowMs = System.currentTimeMillis()
        }
    }
    val seconds = thinkingSeconds(startedAtMs, nowMs)
    Row(verticalAlignment = Alignment.CenterVertically) {
        StatusMark(state = SessionState.Running)
        Spacer(Modifier.width(TetherSpace.space2))
        Text(
            text = stringResource(R.string.states_thinking),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text,
        )
        if (seconds != null) {
            Spacer(Modifier.width(TetherSpace.space2))
            Text(
                text = stringResource(R.string.states_thinking_seconds, seconds),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text2,
            )
        }
    }
}
