pluginManagement {
    includeBuild("build-logic")
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "tether"

include(":app")
include(":core-bridge")
include(":designsystem")

// Proof that detekt fails on a long function. Built only by tools/check-smell-fixtures.sh.
if (providers.gradleProperty("tether.smellFixtures").isPresent) include(":smell-fixtures")
