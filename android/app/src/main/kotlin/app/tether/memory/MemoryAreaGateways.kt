package app.tether.memory

import app.tether.core.generated.MemoryRuntime
import app.tether.core.generated.SkillsRuntime
import app.tether.core.generated.VaultRuntime
import java.io.File

/** The three gateways of the skills and memory area. The app module creates one set for the app. */
class MemoryAreaGateways(
    val memory: MemoryGateway,
    val skills: SkillsGateway,
    val vault: VaultGateway,
)

/** Creates the memory, skills, and vault gateways over the app's files folder. */
fun createMemoryAreaGateways(filesDir: File): MemoryAreaGateways =
    MemoryAreaGateways(
        memory = createMemoryGateway(filesDir),
        skills = createSkillsGateway(filesDir),
        vault = createVaultGateway(filesDir),
    )

/**
 * The memory gateway over `<filesDir>/memory`. The folder is created here, not in the screens.
 * The runtime opens on the first call, off the main thread.
 */
fun createMemoryGateway(filesDir: File): MemoryGateway {
    val dir = filesDir.resolve("memory").apply { mkdirs() }.absolutePath
    return CoreMemoryGateway(lazy { MemoryRuntime(dir) })
}

/** The skills gateway over `<filesDir>/skills`. The folder is created here. */
fun createSkillsGateway(filesDir: File): SkillsGateway {
    val dir = filesDir.resolve("skills").apply { mkdirs() }.absolutePath
    return CoreSkillsGateway(lazy { SkillsRuntime(dir) })
}

/** The vault gateway over `<filesDir>/vault`. The folder is created here. */
fun createVaultGateway(filesDir: File): VaultGateway {
    val dir = filesDir.resolve("vault").apply { mkdirs() }.absolutePath
    return CoreVaultGateway(lazy { VaultRuntime(dir) })
}
