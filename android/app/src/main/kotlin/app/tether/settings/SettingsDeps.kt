package app.tether.settings

import android.content.Context
import app.tether.di.AppModule

/**
 * The settings screens' data sources. The app creates one for its lifetime, so each runtime opens
 * once. The provider, project, and session data come from [module].
 */
class SettingsDeps(
    context: Context,
    module: AppModule,
) {
    /** The local preferences: profile name, app default model and mode, app lock. */
    val store: SettingsStore = PrefsSettingsStore(context)

    /** Project model and mode defaults through the core. */
    val projectDefaults: ProjectDefaultsGateway =
        createProjectDefaultsGateway(module.databasePath, module.workspacesRoot)

    /** The saved model lists through the core. */
    val models: ModelCatalogGateway = createModelCatalogGateway(module.databasePath, module.vault)

    /** Storage sizes and the update cache. */
    val storage: StorageSource =
        FileStorageSource(
            databaseFile = context.getDatabasePath("tether.db"),
            workspaces = context.filesDir.resolve("workspaces"),
            checkpointBlobs = context.filesDir.resolve("blobs"),
            updateCache = context.cacheDir.resolve("updates"),
            workspaceRoot = {
                java.io.File(module.workspaceGateway.location().effectiveRoot)
            },
            secrets = context.filesDir.resolve("secrets"),
            memory = context.filesDir.resolve("memory"),
            skills = context.filesDir.resolve("skills"),
            vault = context.filesDir.resolve("vault"),
            crashReport = context.filesDir.resolve("crash/last.txt"),
        )

    /** Notification state from the system. */
    val notifications: NotificationSource = SystemNotificationSource(context)
}
