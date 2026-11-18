import io.gitlab.arturbosch.detekt.Detekt
import io.gitlab.arturbosch.detekt.extensions.DetektExtension

plugins {
    id("io.gitlab.arturbosch.detekt")
    id("org.jlleitschuh.gradle.ktlint")
}

extensions.configure<DetektExtension> {
    buildUponDefaultConfig = true
    config.setFrom(rootProject.file("config/detekt.yml"))
    source.setFrom(files("src/main/kotlin", "src/test/kotlin", "src/androidTest/kotlin"))
    parallel = true
}

tasks.withType<Detekt>().configureEach {
    jvmTarget = "17"
    exclude("**/generated/**", "**/tokens/Tokens.kt")
}

ktlint {
    android.set(true)
    filter {
        exclude { it.file.path.contains("/generated/") || it.file.path.endsWith("tokens/Tokens.kt") }
    }
}
