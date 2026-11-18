plugins {
    id("tether.android.library")
}

android {
    namespace = "app.tether.core"

    defaultConfig {
        consumerProguardFiles("consumer-rules.pro")
    }
}

dependencies {
    // UniFFI Kotlin bindings call the Rust library through JNA.
    implementation(libs.jna) { artifact { type = "aar" } }
    implementation(libs.coroutines.android)

    testImplementation(libs.junit)
    testImplementation(libs.coroutines.test)
}
