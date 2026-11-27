import org.gradle.api.DefaultTask
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import java.io.File
import javax.inject.Inject

/** Builds the Rust core for Android ABIs with cargo-ndk and writes `lib<name>.so` per ABI. */
abstract class CargoNdkTask
    @Inject
    constructor(
        private val exec: ExecOperations,
    ) : DefaultTask() {
        @get:Input abstract val release: Property<Boolean>

        @get:Input abstract val abis: ListProperty<String>

        @get:Input abstract val packageName: Property<String>

        @get:Input abstract val ndkDir: Property<String>

        @get:Input abstract val rustDir: Property<String>

        @get:InputFiles
        @get:PathSensitive(PathSensitivity.RELATIVE)
        abstract val rustSources: ConfigurableFileCollection

        @get:OutputDirectory abstract val outputDir: DirectoryProperty

        @TaskAction
        fun build() {
            val home = System.getenv("HOME").orEmpty()
            val cargoBin = "$home/.cargo/bin"
            // Gradle finds the program on its own PATH, not on the PATH set below, and an IDE
            // started from the Dock has no ~/.cargo/bin. Use the absolute path when it exists.
            val cargo = File(cargoBin, "cargo").takeIf { it.canExecute() }?.absolutePath ?: "cargo"
            val args =
                buildList {
                    add(cargo)
                    add("ndk")
                    abis.get().forEach {
                        add("-t")
                        add(it)
                    }
                    add("-o")
                    add(outputDir.get().asFile.absolutePath)
                    add("build")
                    add("-p")
                    add(packageName.get())
                    if (release.get()) add("--release")
                }
            exec.exec {
                workingDir(rustDir.get())
                environment("ANDROID_NDK_HOME", ndkDir.get())
                environment("PATH", "$cargoBin:${System.getenv("PATH").orEmpty()}")
                commandLine(args)
            }
        }
    }
