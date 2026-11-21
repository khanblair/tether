package app.tether.home

/** The longest project name, in characters. */
const val PROJECT_NAME_MAX_LENGTH = 80

/** Why a typed project name cannot be used. */
enum class ProjectNameProblem {
    /** More than [PROJECT_NAME_MAX_LENGTH] characters after trimming. */
    TooLong,

    /** A control character such as a newline or a tab, inside the name. */
    ControlCharacter,
}

/** The result of checking a typed project name. */
sealed interface ProjectNameCheck {
    /** Nothing is typed, or only spaces. The field shows no message, and Create stays off. */
    data object Empty : ProjectNameCheck

    /** A usable name. [name] is trimmed. */
    data class Valid(
        val name: String,
    ) : ProjectNameCheck

    /** A name that cannot be used, with the reason. */
    data class Invalid(
        val problem: ProjectNameProblem,
    ) : ProjectNameCheck
}

/**
 * Checks a typed project name. The name is trimmed first, so leading and trailing spaces, tabs,
 * and line breaks are removed. It must then have 1 to 80 characters, counted by code point, and no
 * control character. A length problem is reported before a control character.
 */
fun checkProjectName(raw: String): ProjectNameCheck {
    val name = raw.trim()
    return when {
        name.isEmpty() -> ProjectNameCheck.Empty
        name.codePointCount(0, name.length) > PROJECT_NAME_MAX_LENGTH ->
            ProjectNameCheck.Invalid(ProjectNameProblem.TooLong)
        name.any(
            Char::isISOControl,
        ) -> ProjectNameCheck.Invalid(ProjectNameProblem.ControlCharacter)
        else -> ProjectNameCheck.Valid(name)
    }
}
