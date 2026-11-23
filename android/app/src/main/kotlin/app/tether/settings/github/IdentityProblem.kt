package app.tether.settings.github

/** Why a Git identity field cannot be saved. Mirrors the core's identity check. */
enum class IdentityProblem {
    /** The field is empty or only spaces. */
    Empty,

    /** The field has `<`, `>`, or a line break. */
    Forbidden,
}
