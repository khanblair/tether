import com.android.build.api.dsl.CommonExtension
import org.gradle.api.JavaVersion
import org.gradle.api.Project

/** Settings shared by every Android module. Targets are listed in docs/development.md section 10. */
internal const val COMPILE_SDK = 37
internal const val MIN_SDK = 29
internal const val TARGET_SDK = 36

internal fun Project.configureAndroidCommon(android: CommonExtension) {
    android.compileSdk = COMPILE_SDK
    android.compileOptions.sourceCompatibility = JavaVersion.VERSION_17
    android.compileOptions.targetCompatibility = JavaVersion.VERSION_17
    android.lint.warningsAsErrors = true
    android.lint.abortOnError = true
    android.lint.lintConfig = rootProject.file("config/lint.xml")
}
