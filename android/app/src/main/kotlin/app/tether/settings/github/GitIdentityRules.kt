package app.tether.settings.github

/**
 * The problem with one identity field, or null when it can be saved. The forbidden check runs on
 * the value as typed, the same as the core does.
 */
fun identityProblem(value: String): IdentityProblem? =
    when {
        value.isBlank() -> IdentityProblem.Empty
        value.any {
            it == '<' || it == '>' || it == '\n' || it == '\r'
        } -> IdentityProblem.Forbidden
        else -> null
    }
