package app.tether.settings.executors

/**
 * What went wrong in a call to the core. Each value is a fixed word, so no message carries a
 * host path, a key, or a token.
 */
enum class ExecutorsFailure {
    /** This build has no executor path or verifying key for the call. */
    NotConfigured,

    /** Local settings, storage, or a runtime folder could not be read or written. */
    Unavailable,

    /** The project does not exist. */
    ProjectNotFound,

    /** The project has no repository on this phone. */
    NoRepository,

    /** GitHub sign-in is missing. */
    NotSignedIn,

    /** The project has no GitHub remote. */
    NoRemote,

    /** GitHub returned an error. The detail is never shown. */
    GithubError,

    /** A downloaded pack or root filesystem failed its check. */
    VerificationFailed,

    /** Any other failure. */
    Unknown,
}

/** Thrown by [ExecutorsGateway] calls. Its message is the failure name only. */
class ExecutorsCallFailed(
    val failure: ExecutorsFailure,
) : Exception(failure.name)

/** Reports byte progress of a download. [total] is 0 when the size is not known yet. */
typealias ProgressReport = (downloaded: Long, total: Long) -> Unit

/** One runtime pack version, as the sandbox section needs it. */
data class PackSummary(
    val id: String,
    val version: String,
    val installed: Boolean,
    val sizeBytes: Long,
)

/** Whether CI can run for one project, from the core's status. */
enum class CiReadiness {
    /** Signed in, a GitHub remote, and a current workflow file. */
    Ready,

    /** GitHub sign-in is missing. */
    NotSignedIn,

    /** The project has no GitHub remote. */
    NoRemote,

    /** The workflow file is not in the project. */
    WorkflowMissing,

    /** The workflow file is older than this app expects. */
    WorkflowOutdated,

    /** The core could not tell, for any other reason. */
    Unavailable,
}

/** One workflow run from the core's run list. Times and states are the raw GitHub values. */
data class CiRunSummary(
    val id: Long,
    val title: String,
    val branch: String,
    val status: String,
    val conclusion: String,
    val startedAt: String,
    val durationSeconds: Long,
    val htmlUrl: String,
)

/**
 * Where this build gets its executor files. A null manifest URL means this build has none for
 * that part, so its button is shown as not available.
 */
data class ExecutorBuildConfig(
    val packManifestUrl: String?,
    val rootfsManifestUrl: String?,
    val linuxSupported: Boolean,
)
