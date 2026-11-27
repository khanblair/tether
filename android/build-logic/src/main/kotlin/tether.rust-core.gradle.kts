import com.android.build.api.dsl.ApplicationExtension
import com.android.build.api.variant.ApplicationAndroidComponentsExtension

// Builds the Rust core with cargo-ndk and packages libtether_core.so for every ABI.
// Needs: rustup targets from rust-toolchain.toml, cargo-ndk, and the pinned NDK.

extensions.configure<ApplicationExtension> {
    ndkVersion = providers.gradleProperty("tether.ndkVersion").get()
}

val rustCoreDir = rootProject.layout.projectDirectory.dir("../core")
val targetAbis = tetherAbis(providers)

extensions.configure<ApplicationAndroidComponentsExtension> {
    onVariants { variant ->
        val isRelease = variant.buildType == "release"
        val taskName = if (isRelease) "cargoNdkRelease" else "cargoNdkDebug"
        val cargoTask =
            if (tasks.names.contains(taskName)) {
                tasks.named<CargoNdkTask>(taskName)
            } else {
                tasks.register<CargoNdkTask>(taskName) {
                    group = "rust"
                    description = "Builds libtether_core.so for ${targetAbis.joinToString()}."
                    release.set(isRelease)
                    abis.set(targetAbis)
                    packageName.set("tether-ffi")
                    ndkDir.set(sdkComponents.ndkDirectory.map { it.asFile.absolutePath })
                    rustDir.set(rustCoreDir.asFile.absolutePath)
                    rustSources.from(
                        rustCoreDir.asFileTree.matching {
                            include("Cargo.toml", "Cargo.lock", "**/Cargo.toml", "**/*.rs", "**/uniffi.toml")
                            exclude("target/**", "fixtures/**")
                        },
                    )
                    outputDir.set(layout.buildDirectory.dir("rustJniLibs/$taskName"))
                }
            }
        variant.sources.jniLibs?.addGeneratedSourceDirectory(cargoTask, CargoNdkTask::outputDir)
    }
}
