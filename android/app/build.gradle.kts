plugins {
    id("tether.android.application")
    id("tether.compose")
    id("tether.rust-core")
}

// The app version is the core version: one number in core/Cargo.toml, read here.
val coreVersion: String =
    rootProject
        .file("../core/Cargo.toml")
        .readLines()
        .firstNotNullOf {
            Regex(
                """^version\s*=\s*"(\d+\.\d+\.\d+)"""",
            ).find(it)?.groupValues?.get(1)
        }
val (major, minor, patch) = coreVersion.split(".").map { it.toInt() }

android {
    namespace = "app.tether"

    defaultConfig {
        applicationId = "io.github.khanblair.tether"
        // MAJOR * 1_000_000 + MINOR * 10_000 + PATCH * 100 + ABI, where ABI is 0 for a universal APK.
        versionCode = major * 1_000_000 + minor * 10_000 + patch * 100
        versionName = coreVersion
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    flavorDimensions += "distribution"
    productFlavors {
        // Google Play build. Linux mode ships here only if Play policy allows it (open question Q1).
        create("play") {
            dimension = "distribution"
        }
        // GitHub Releases build with every feature. Installs next to the Play build.
        create("full") {
            dimension = "distribution"
            applicationIdSuffix = ".full"
        }
    }

    signingConfigs {
        // Release signing reads the environment; see docs/development.md section 5.
        val keystorePath = System.getenv("TETHER_KEYSTORE_PATH")
        if (!keystorePath.isNullOrBlank()) {
            create("release") {
                storeFile = file(keystorePath)
                storePassword = System.getenv("TETHER_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("TETHER_KEY_ALIAS")
                keyPassword = System.getenv("TETHER_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            signingConfig = signingConfigs.findByName("release")
        }
    }
}

// A local run can narrow the ABIs with tether.abis (see TetherAbis.kt). The filter above stays
// literal because Android lint checks it for x86_64 support, so the other ABIs are dropped here.
androidComponents {
    onVariants { variant ->
        val kept = tetherAbis(providers)
        DEFAULT_ABIS.filterNot { it in kept }.forEach { abi ->
            variant.packaging.jniLibs.excludes
                .add("**/$abi/*.so")
        }
    }
}

dependencies {
    implementation(project(":core-bridge"))
    implementation(project(":designsystem"))

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.navigation.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.coroutines.android)

    testImplementation(libs.junit)
    testImplementation(libs.coroutines.test)

    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.test.runner)
}
