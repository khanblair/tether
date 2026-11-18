import com.android.build.api.dsl.LibraryExtension

plugins {
    id("com.android.library")
    id("tether.quality")
}

extensions.configure<LibraryExtension> {
    configureAndroidCommon(this)
    defaultConfig {
        minSdk = MIN_SDK
    }
}
