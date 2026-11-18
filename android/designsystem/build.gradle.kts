plugins {
    id("tether.android.library")
    id("tether.compose")
}

android {
    namespace = "app.tether.design"
}

dependencies {
    implementation(libs.compose.animation.core)

    testImplementation(libs.junit)
}
