// Root build file. Module setup lives in build-logic convention plugins.

tasks.register("qualityCheck") {
    group = "verification"
    description = "Runs detekt and ktlint in every module."
    dependsOn(subprojects.map { "${it.path}:detekt" }, subprojects.map { "${it.path}:ktlintCheck" })
}
