package app.tether.session

import app.tether.core.generated.ProviderView
import app.tether.core.generated.SessionStatusView

/** A session gateway for unit tests. It keeps sessions in memory and records each creation. */
class FakeSessionGateway : SessionGateway {
    val sessionsByProject = mutableMapOf<String, List<SessionInfo>>()
    val models = mutableMapOf<ProviderView, String?>()
    val created = mutableListOf<Triple<String, ProviderView, String>>()

    /** The id [createSession] returns. */
    var nextSessionId = "new-session"

    /** Throws this from the next call of any kind, then clears itself. */
    var failNext: Exception? = null

    private fun maybeFail() {
        failNext?.let {
            failNext = null
            throw it
        }
    }

    override fun listSessions(projectId: String): List<SessionInfo> {
        maybeFail()
        return sessionsByProject[projectId].orEmpty()
    }

    override fun defaultModel(provider: ProviderView): String? {
        maybeFail()
        return models[provider]
    }

    override fun createSession(
        projectId: String,
        provider: ProviderView,
        modelId: String,
    ): String {
        maybeFail()
        created += Triple(projectId, provider, modelId)
        return nextSessionId
    }
}

/** A session in the given status, for tests of the lists and the status mapping. */
fun sessionInfo(
    id: String,
    status: SessionStatusView = SessionStatusView.IDLE,
    title: String = "Session $id",
    updatedAtMs: Long = 0L,
) = SessionInfo(id = id, title = title, status = status, updatedAtMs = updatedAtMs)
