package app.tether.session.approval

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.session.agent.AgentLoopException
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.AgentLoopGateway
import app.tether.session.agent.ApprovalUi
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** The longest reason a denial can carry, in characters. */
const val MAX_DENY_REASON_CHARS = 200

/** The sheet open over the approval bar. It names the approval it is about. */
sealed interface ApprovalSheet {
    /** The approval this sheet answers. */
    val toolCallId: String

    /** The Always allow menu for [toolCallId]. */
    data class AlwaysAllow(
        override val toolCallId: String,
    ) : ApprovalSheet

    /** The deny sheet for [toolCallId], with its optional reason. */
    data class Deny(
        override val toolCallId: String,
    ) : ApprovalSheet
}

/**
 * What the approval screens show. [approvals] is the waiting list from the agent loop. [expandedId]
 * and [sheet] name an approval, and they clear when that approval leaves the list. [busy] is true
 * while a call runs, and [failure] is the last failed call.
 */
data class ApprovalScreenState(
    val approvals: List<ApprovalUi> = emptyList(),
    val expandedId: String? = null,
    val sheet: ApprovalSheet? = null,
    val reason: String = "",
    val busy: Boolean = false,
    val failure: AgentLoopFailure? = null,
)

/**
 * Answers the approvals of one session. It does not own the agent state: [approvals] comes from
 * the agent loop, and [onResolved] reports each approval that the core accepted. Gateway calls run
 * on [io]. A failed call sets [ApprovalScreenState.failure] and calls [onResolved] for nothing.
 */
class ApprovalViewModel(
    private val sessionId: String,
    private val approvals: StateFlow<List<ApprovalUi>>,
    private val gateway: AgentLoopGateway,
    private val onResolved: (String) -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val local = MutableStateFlow(ApprovalScreenState())

    /** The screen state. */
    val state: StateFlow<ApprovalScreenState> =
        combine(this.approvals, local) { waiting, current -> current.withApprovals(waiting) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, ApprovalScreenState())

    /** Shows the full text of [toolCallId], or hides it when it is already shown. */
    fun toggleExpanded(toolCallId: String) {
        local.update {
            it.copy(expandedId = if (it.expandedId == toolCallId) null else toolCallId)
        }
    }

    /** Opens the Always allow menu. It opens only when the call gets a rule. */
    fun openAlwaysAllow(toolCallId: String) {
        val approval = find(toolCallId) ?: return
        alwaysAllowRule(approval.tool, approval.target) ?: return
        local.update { it.copy(sheet = ApprovalSheet.AlwaysAllow(toolCallId), failure = null) }
    }

    /** Opens the deny sheet with an empty reason. */
    fun openDeny(toolCallId: String) {
        find(toolCallId) ?: return
        local.update {
            it.copy(sheet = ApprovalSheet.Deny(toolCallId), reason = "", failure = null)
        }
    }

    /** Sets the denial reason. The text is cut to [MAX_DENY_REASON_CHARS]. */
    fun editReason(text: String) {
        local.update { it.copy(reason = text.take(MAX_DENY_REASON_CHARS)) }
    }

    /** Closes the open sheet and drops its reason. */
    fun closeSheet() {
        local.update { it.copy(sheet = null, reason = "", failure = null) }
    }

    /** Allows the call [toolCallId] once. */
    fun allow(toolCallId: String) {
        resolve(toolCallId) { gateway.approve(sessionId, toolCallId, always = false) }
    }

    /**
     * Allows the call [toolCallId] and every future call that matches its rule. Nothing happens
     * when the call gets no rule.
     */
    fun confirmAlwaysAllow(toolCallId: String) {
        val approval = find(toolCallId) ?: return
        alwaysAllowRule(approval.tool, approval.target) ?: return
        resolve(toolCallId) { gateway.approve(sessionId, toolCallId, always = true) }
    }

    /** Denies the call [toolCallId]. A blank reason denies without one. */
    fun confirmDeny(toolCallId: String) {
        val reason =
            local.value.reason
                .trim()
                .ifEmpty { null }
        resolve(toolCallId) { gateway.deny(sessionId, toolCallId, reason) }
    }

    private fun find(toolCallId: String): ApprovalUi? =
        approvals.value.firstOrNull { it.toolCallId == toolCallId }

    /** Runs one gateway call. A second call waits until the first one ends. */
    private fun resolve(
        toolCallId: String,
        call: () -> Unit,
    ) {
        if (local.value.busy) return
        local.update { it.copy(busy = true, failure = null) }
        viewModelScope.launch {
            val failure = attempt(call)
            if (failure == null) {
                local.update { it.copy(busy = false, sheet = null, reason = "") }
                onResolved(toolCallId)
            } else {
                local.update { it.copy(busy = false, failure = failure) }
            }
        }
    }

    private suspend fun attempt(call: () -> Unit): AgentLoopFailure? =
        try {
            withContext(io) { call() }
            null
        } catch (error: AgentLoopException) {
            error.reason
        }
}

/** Drops the expanded id and the sheet when their approval is no longer waiting. */
private fun ApprovalScreenState.withApprovals(waiting: List<ApprovalUi>): ApprovalScreenState =
    copy(
        approvals = waiting,
        expandedId = expandedId?.takeIf { id -> waiting.any { it.toolCallId == id } },
        sheet = sheet?.takeIf { open -> waiting.any { it.toolCallId == open.toolCallId } },
    )
