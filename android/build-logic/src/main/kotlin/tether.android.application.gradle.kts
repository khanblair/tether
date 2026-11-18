import com.android.build.api.dsl.ApplicationExtension

plugins {
    id("com.android.application")
    id("tether.quality")
}

extensions.configure<ApplicationExtension> {
    configureAndroidCommon(this)
    defaultConfig {
        minSdk = MIN_SDK
        targetSdk = TARGET_SDK
    }
}
