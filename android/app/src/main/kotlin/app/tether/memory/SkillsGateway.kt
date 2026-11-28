package app.tether.memory

import app.tether.core.generated.SkillsRuntime
import app.tether.core.generated.SkillsRuntimeException

/** One skill folder as the list shows it. An invalid skill has an [error] and no parsed name. */
data class SkillItem(
    val folder: String,
    val name: String,
    val description: String,
    val valid: Boolean,
    val error: String?,
)

/** One parsed skill with its body, read only. */
data class SkillDetail(
    val folder: String,
    val name: String,
    val description: String,
    val license: String?,
    val compatibility: String?,
    val allowedTools: String?,
    val metadata: Map<String, String>,
    val body: String,
)

/** One bundled sample skill and whether it is already in the user skills folder. */
data class SampleSkillItem(
    val name: String,
    val description: String,
    val fileCount: Int,
    val installed: Boolean,
)

/** What the skill screens need from the Rust core. Calls block, so run them off the main thread. */
interface SkillsGateway {
    /** Every skill folder, valid or not. */
    fun list(): List<SkillItem>

    /** The parsed skill in [folder]. */
    fun read(folder: String): SkillDetail

    /** The bundled sample skills, with their install state. */
    fun samples(): List<SampleSkillItem>

    /** Copies the sample [name] into the user skills folder. A second install changes nothing. */
    fun installSample(name: String)

    /** The sources and licenses text of the sample skills. */
    fun samplesNotice(): String
}

/** The real gateway over the UniFFI [SkillsRuntime]. Core errors leave as [AreaFailure]. */
class CoreSkillsGateway(
    private val lazyRuntime: Lazy<SkillsRuntime>,
) : SkillsGateway {
    private val runtime: SkillsRuntime by lazyRuntime

    override fun list(): List<SkillItem> =
        guard {
            runtime.list().map { view ->
                SkillItem(
                    folder = view.folder,
                    name = view.name,
                    description = view.description,
                    valid = view.valid,
                    error = view.error,
                )
            }
        }

    override fun read(folder: String): SkillDetail =
        guard {
            runtime.read(folder).let { view ->
                SkillDetail(
                    folder = view.folder,
                    name = view.name,
                    description = view.description,
                    license = view.license,
                    compatibility = view.compatibility,
                    allowedTools = view.allowedTools,
                    metadata = view.metadata,
                    body = view.body,
                )
            }
        }

    override fun samples(): List<SampleSkillItem> =
        guard {
            runtime.sampleSkills().map { view ->
                SampleSkillItem(
                    name = view.name,
                    description = view.description,
                    fileCount = view.fileCount.toInt(),
                    installed = view.installed,
                )
            }
        }

    override fun installSample(name: String) = guard { runtime.installSampleSkill(name) }

    override fun samplesNotice(): String = guard { runtime.sampleSkillsNotice() }

    private inline fun <T> guard(block: () -> T): T =
        try {
            block()
        } catch (failure: SkillsRuntimeException) {
            throw AreaFailure(failure.problem(), failure)
        }
}
