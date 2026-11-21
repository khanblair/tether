package app.tether.memory

import androidx.annotation.StringRes
import app.tether.R

/** The sentence that explains [this] problem. Each one says what happened and what to do. */
@StringRes
internal fun AreaProblem.messageRes(): Int =
    when (this) {
        AreaProblem.InvalidPath -> R.string.memory_problem_path
        AreaProblem.NotFound -> R.string.memory_problem_not_found
        AreaProblem.OverCap -> R.string.memory_problem_over_cap
        AreaProblem.InvalidEntry -> R.string.memory_problem_invalid_entry
        AreaProblem.UnsafeContent -> R.string.memory_problem_unsafe
        AreaProblem.InvalidNote -> R.string.vault_problem_invalid_note
        AreaProblem.InvalidSkill -> R.string.skills_problem_invalid
        AreaProblem.Unavailable -> R.string.area_problem_unavailable
    }
