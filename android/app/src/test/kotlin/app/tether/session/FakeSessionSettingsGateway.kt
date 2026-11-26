package app.tether.session

import app.tether.core.generated.ProviderView

/**
 * A settings gateway for unit tests. It keeps one session and a model list per provider in
 * memory, and records each change. Set [failNext] to make the next call throw.
 */
class FakeSessionSettingsGateway(
    var session: SessionDetails =
        SessionDetails(
            title = "Fix login",
            provider = ProviderView.ANTHROPIC,
            modelId = "sonnet",
            mode = SessionMode.ACCEPT_EDITS,
        ),
) : SessionSettingsGateway {
    val modelsByProvider = mutableMapOf<ProviderView, List<ModelChoice>>()
    var usageValue = SessionUsage(tokensIn = 1_200, tokensOut = 400, costMicros = 0)
    var checkpointList = emptyList<CheckpointItem>()
    var sessionHits = emptyList<SearchHit>()
    var transcriptHits = emptyList<SearchHit>()
    val calls = mutableListOf<String>()

    /** Whether the fake session has Bypass on. */
    var bypass = false

    /** Throws this from the next call of any kind, then clears itself. */
    var failNext: Exception? = null

    private fun record(call: String) {
        calls += call
        failNext?.let {
            failNext = null
            throw it
        }
    }

    override fun details(sessionId: String): SessionDetails {
        record("details")
        return session
    }

    override fun models(provider: ProviderView): List<ModelChoice> {
        record("models")
        return modelsByProvider[provider].orEmpty()
    }

    override fun setModel(
        sessionId: String,
        provider: ProviderView,
        modelId: String,
    ) {
        record("setModel:${provider.name}:$modelId")
        session = session.copy(provider = provider, modelId = modelId)
    }

    override fun setMode(
        sessionId: String,
        mode: SessionMode,
    ) {
        record("setMode:${mode.name}")
        session = session.copy(mode = mode)
    }

    override fun rename(
        sessionId: String,
        title: String,
    ) {
        record("rename:$title")
        session = session.copy(title = title)
    }

    override fun bypassActive(sessionId: String): Boolean {
        record("bypassActive")
        return bypass
    }

    override fun enableBypass(sessionId: String) {
        record("enableBypass")
        bypass = true
    }

    override fun disableBypass(sessionId: String) {
        record("disableBypass")
        bypass = false
    }

    override fun setEffort(
        sessionId: String,
        effort: ThinkingEffort?,
    ) {
        record("setEffort:${effort?.name ?: "none"}")
        session = session.copy(effort = effort)
    }

    override fun archive(
        sessionId: String,
        archived: Boolean,
    ) {
        record("archive:$archived")
    }

    override fun delete(sessionId: String) {
        record("delete")
    }

    override fun usage(sessionId: String): SessionUsage {
        record("usage")
        return usageValue
    }

    override fun checkpoints(sessionId: String): List<CheckpointItem> {
        record("checkpoints")
        return checkpointList
    }

    override fun searchSessions(
        projectId: String,
        query: String,
    ): List<SearchHit> {
        record("searchSessions:$projectId:$query")
        return sessionHits
    }

    override fun searchTranscripts(
        phrase: String,
        limit: Int,
    ): List<SearchHit> {
        record("searchTranscripts:$phrase:$limit")
        return transcriptHits
    }
}

/** A rewinder for unit tests. It records each restore and can fail the next one. */
class FakeSessionRewinder : SessionRewinder {
    val restores = mutableListOf<Pair<String, RewindScope>>()
    var failNext: Exception? = null

    /** The message ids each message rewind reached, in order. */
    val rewoundToMessages = mutableListOf<String>()

    /** The next message rewind throws this, if set. It is cleared after it throws. */
    var failNextRewindToMessage: Exception? = null

    override fun rewind(
        checkpointId: String,
        scope: RewindScope,
    ) {
        restores += checkpointId to scope
        failNext?.let {
            failNext = null
            throw it
        }
    }

    override fun rewindToMessage(messageId: String) {
        rewoundToMessages += messageId
        failNextRewindToMessage?.let {
            failNextRewindToMessage = null
            throw it
        }
    }
}
