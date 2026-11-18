plugins {
    id("org.jetbrains.kotlin.plugin.compose")
}

val catalog = extensions.getByType<VersionCatalogsExtension>().named("libs")

dependencies {
    add("implementation", platform(catalog.findLibrary("compose-bom").get()))
    add("implementation", catalog.findLibrary("compose-ui").get())
    add("implementation", catalog.findLibrary("compose-material3").get())
    add("implementation", catalog.findLibrary("compose-ui-tooling-preview").get())
    add("debugImplementation", catalog.findLibrary("compose-ui-tooling").get())
}
