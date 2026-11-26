package app.tether.settings

import app.tether.core.generated.ProviderView
import app.tether.session.SessionMode
import app.tether.session.ThinkingEffort

/** Settings kept in memory. Every write is visible to the test. */
class FakeSettingsStore(
    var name: String = "",
    var model: DefaultModel? = null,
    var mode: SessionMode = SessionMode.ACCEPT_EDITS,
    var lock: Boolean = false,
    var finished: Boolean = false,
    var effort: ThinkingEffort? = ThinkingEffort.MEDIUM,
) : SettingsStore {
    override fun profileName(): String = name

    override fun saveProfileName(name: String) {
        this.name = name
    }

    override fun defaultModel(): DefaultModel? = model

    override fun saveDefaultModel(model: DefaultModel?) {
        this.model = model
    }

    override fun defaultMode(): SessionMode = mode

    override fun savedDefaultMode(): SessionMode? = mode

    override fun saveDefaultMode(mode: SessionMode) {
        this.mode = mode
    }

    override fun defaultEffort(): ThinkingEffort? = effort

    override fun savedDefaultEffort(): ThinkingEffort? = effort

    override fun saveDefaultEffort(effort: ThinkingEffort?) {
        this.effort = effort
    }

    override fun onboarded(): Boolean = finished

    override fun saveOnboarded(done: Boolean) {
        finished = done
    }

    override fun appLockEnabled(): Boolean = lock

    override fun saveAppLockEnabled(enabled: Boolean) {
        lock = enabled
    }
}

/** Project defaults in memory. [failLoad] makes every read throw, to test the failure states. */
class FakeProjectDefaults(
    val saved: MutableMap<String, ProjectDefaults> = mutableMapOf(),
) : ProjectDefaultsGateway {
    val writes = mutableListOf<Pair<String, ProjectDefaults>>()
    var failLoad = false

    override fun load(projectId: String): ProjectDefaults {
        check(!failLoad) { "load failed" }
        return saved[projectId] ?: ProjectDefaults(null, null, null)
    }

    override fun save(
        projectId: String,
        defaults: ProjectDefaults,
    ) {
        saved[projectId] = defaults
        writes += projectId to defaults
    }
}

/** Saved models per provider. A provider missing from the map has an empty list. */
class FakeModelCatalog(
    val lists: Map<ProviderView, List<CatalogModel>> = emptyMap(),
    var fail: Boolean = false,
) : ModelCatalogGateway {
    override fun models(provider: ProviderView): List<CatalogModel> {
        check(!fail) { "catalog failed" }
        return lists[provider].orEmpty()
    }
}

/** Storage sizes in memory. The clear result is set by the test. */
class FakeStorage(
    var usage: StorageUsage = StorageUsage(1L, 2L, 3L, 4L),
    var clearResult: Boolean = true,
) : StorageSource {
    var clears = 0

    override fun usage(): StorageUsage = usage

    override fun clearUpdateCache(): Boolean {
        clears++
        return clearResult
    }
}

/** Notification state in memory. Channel importance is set per id; a missing id has none. */
class FakeNotifications(
    var enabled: Boolean = true,
    var canAsk: Boolean = false,
    val importances: MutableMap<String, Int> = mutableMapOf(),
) : NotificationSource {
    override fun enabled(): Boolean = enabled

    override fun importance(channelId: String): Int? = importances[channelId]

    override fun canAskForPermission(): Boolean = canAsk
}
