package app.tether.memory

import app.tether.core.generated.MemoryRuntimeException
import app.tether.core.generated.SkillsRuntimeException
import app.tether.core.generated.VaultRuntimeException

/** Why a memory, skill, or vault call failed, in the terms the screens need to explain it. */
enum class AreaProblem {
    /** The path is outside the allowed folders or is not a Markdown path. */
    InvalidPath,

    /** The entry, note, or skill no longer exists. */
    NotFound,

    /** The result would go over the character cap of the file. */
    OverCap,

    /** The memory entry is empty or contains the section sign that separates entries. */
    InvalidEntry,

    /** The entry has blocked content: an instruction, a credential, or a hidden character. */
    UnsafeContent,

    /** The note has no description, or its frontmatter does not parse. */
    InvalidNote,

    /** The skill file could not be read. */
    InvalidSkill,

    /** The files on this phone cannot be read or written. */
    Unavailable,
}

/** A core call failed. [problem] says which message to show; [cause] is the core error. */
class AreaFailure(
    val problem: AreaProblem,
    cause: Throwable? = null,
) : Exception(cause)

/** The problem behind a failed call. Any other failure counts as unavailable. */
internal fun Throwable.areaProblem(): AreaProblem =
    (this as? AreaFailure)?.problem ?: AreaProblem.Unavailable

internal fun MemoryRuntimeException.problem(): AreaProblem =
    when (this) {
        is MemoryRuntimeException.InvalidPath -> AreaProblem.InvalidPath
        is MemoryRuntimeException.NotFound -> AreaProblem.NotFound
        is MemoryRuntimeException.OverCap -> AreaProblem.OverCap
        is MemoryRuntimeException.InvalidEntry -> AreaProblem.InvalidEntry
        is MemoryRuntimeException.UnsafeContent -> AreaProblem.UnsafeContent
        is MemoryRuntimeException.Unavailable -> AreaProblem.Unavailable
    }

internal fun SkillsRuntimeException.problem(): AreaProblem =
    when (this) {
        is SkillsRuntimeException.InvalidPath -> AreaProblem.InvalidPath
        is SkillsRuntimeException.NotFound -> AreaProblem.NotFound
        is SkillsRuntimeException.InvalidSkill -> AreaProblem.InvalidSkill
        is SkillsRuntimeException.Unavailable -> AreaProblem.Unavailable
        // The install calls have no screen yet; they map to the nearest existing explanation.
        is SkillsRuntimeException.InvalidUrl -> AreaProblem.InvalidPath
        is SkillsRuntimeException.NotASkill,
        is SkillsRuntimeException.SeveralSkills,
        is SkillsRuntimeException.TooLarge,
        is SkillsRuntimeException.AlreadyInstalled,
        -> AreaProblem.InvalidSkill
        is SkillsRuntimeException.Blocked -> AreaProblem.UnsafeContent
        is SkillsRuntimeException.Download,
        is SkillsRuntimeException.NotTrusted,
        -> AreaProblem.Unavailable
    }

internal fun VaultRuntimeException.problem(): AreaProblem =
    when (this) {
        is VaultRuntimeException.InvalidPath -> AreaProblem.InvalidPath
        is VaultRuntimeException.InvalidNote -> AreaProblem.InvalidNote
        is VaultRuntimeException.NotFound -> AreaProblem.NotFound
        is VaultRuntimeException.OverCap -> AreaProblem.OverCap
        is VaultRuntimeException.Unavailable -> AreaProblem.Unavailable
        // Export, import, and sync have no screen yet.
        is VaultRuntimeException.Archive,
        is VaultRuntimeException.Cancelled,
        is VaultRuntimeException.VaultNotEmpty,
        -> AreaProblem.Unavailable
    }
