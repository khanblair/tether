import org.gradle.api.provider.ProviderFactory

/** The ABIs Tether builds for when nothing narrows them. CI and releases always use these. */
val DEFAULT_ABIS = listOf("arm64-v8a", "x86_64")

/**
 * The ABIs to build the Rust core and package for. The Gradle property `tether.abis` narrows them
 * for local runs, for example `tether.abis=arm64-v8a` in `~/.gradle/gradle.properties` on an
 * Apple silicon emulator. A debug APK then holds one core library instead of two.
 */
fun tetherAbis(providers: ProviderFactory): List<String> =
    providers
        .gradleProperty("tether.abis")
        .map { value -> value.split(",").map(String::trim).filter(String::isNotEmpty) }
        .orElse(DEFAULT_ABIS)
        .get()
        .ifEmpty { DEFAULT_ABIS }
