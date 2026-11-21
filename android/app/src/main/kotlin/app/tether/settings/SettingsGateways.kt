package app.tether.settings

import app.tether.core.generated.PermissionModeView
import app.tether.core.generated.ProjectRuntime
import app.tether.core.generated.ProjectRuntimeException
import app.tether.core.generated.ProviderView
import app.tether.core.generated.SecretVault
import app.tether.core.generated.SessionRuntime
import app.tether.core.generated.SessionRuntimeException
import app.tether.session.QuietEvents
import app.tether.session.SessionMode

/** A core call failed. The screens show a plain message; [cause] holds the core's error. */
class SettingsCallFailed(
    cause: Throwable,
) : Exception(cause)

/**
 * A project's saved defaults. Null means the project has no default for that field. The core
 * stores the three fields together, so a save writes all of them.
 */
data class ProjectDefaults(
    val provider: ProviderView?,
    val modelId: String?,
    val mode: SessionMode?,
)

/** Reads and writes a project's model and mode defaults through the core. Calls block. */
interface ProjectDefaultsGateway {
    /** The saved defaults of [projectId]. */
    fun load(projectId: String): ProjectDefaults

    /** Saves [defaults] for [projectId], replacing all three fields. */
    fun save(
        projectId: String,
        defaults: ProjectDefaults,
    )
}

/** The real gateway over the UniFFI [ProjectRuntime]. The runtime opens on the first call. */
class CoreProjectDefaultsGateway(
    lazyRuntime: Lazy<ProjectRuntime>,
) : ProjectDefaultsGateway {
    private val runtime: ProjectRuntime by lazyRuntime

    override fun load(projectId: String): ProjectDefaults =
        guardProject {
            runtime.loadProject(projectId).let { project ->
                ProjectDefaults(
                    provider = project.defaultProvider,
                    modelId = project.defaultModelId,
                    mode = project.defaultMode?.let { SessionMode.valueOf(it.name) },
                )
            }
        }

    override fun save(
        projectId: String,
        defaults: ProjectDefaults,
    ) = guardProject {
        runtime.setProjectDefaults(
            projectId,
            defaults.provider,
            defaults.modelId,
            defaults.mode?.let { PermissionModeView.valueOf(it.name) },
        )
    }

    private inline fun <T> guardProject(block: () -> T): T =
        try {
            block()
        } catch (error: ProjectRuntimeException) {
            throw SettingsCallFailed(error)
        }
}

/**
 * Opens the project defaults gateway. The runtime is created on the first call, off the main
 * thread.
 */
fun createProjectDefaultsGateway(
    databasePath: String,
    workspacesRoot: String,
): ProjectDefaultsGateway =
    CoreProjectDefaultsGateway(lazy { ProjectRuntime(databasePath, workspacesRoot) })

/** One saved model, with the provider it belongs to. */
data class CatalogModel(
    val provider: ProviderView,
    val id: String,
    val displayName: String,
)

/** The saved model lists of the providers. Calls block. */
interface ModelCatalogGateway {
    /** The saved models of [provider], in the catalog order. Empty when none are saved. */
    fun models(provider: ProviderView): List<CatalogModel>
}

/** The real gateway over the UniFFI [SessionRuntime]'s `listModels` call. */
class CoreModelCatalogGateway(
    lazyRuntime: Lazy<SessionRuntime>,
) : ModelCatalogGateway {
    private val runtime: SessionRuntime by lazyRuntime

    override fun models(provider: ProviderView): List<CatalogModel> =
        try {
            runtime.listModels(provider).map { model ->
                CatalogModel(provider, model.id, model.displayName)
            }
        } catch (error: SessionRuntimeException) {
            throw SettingsCallFailed(error)
        }
}

/** Opens the model catalog gateway. The runtime is created on the first call. */
fun createModelCatalogGateway(
    databasePath: String,
    vault: SecretVault,
): ModelCatalogGateway =
    CoreModelCatalogGateway(lazy { SessionRuntime(databasePath, vault, QuietEvents) })
