package app.tether.memory

import app.tether.core.generated.VaultLinkView
import app.tether.core.generated.VaultNoteView
import app.tether.core.generated.VaultRuntime
import app.tether.core.generated.VaultRuntimeException

/** A note in the vault, with the fields the screens show. [source] is the whole Markdown file. */
data class VaultNote(
    val path: String,
    val title: String,
    val description: String,
    val noteType: String?,
    val tags: List<String>,
    val aliases: List<String>,
    val updated: String?,
    val headings: List<String>,
    val source: String,
)

/**
 * A link written in a note. [resolved] is false when no note matches its target. [resolvedPath] is
 * the note path the core resolved the target to, or null when none matches.
 */
data class VaultLink(
    val from: String,
    val target: String,
    val heading: String?,
    val alias: String?,
    val resolved: Boolean,
    val resolvedPath: String?,
)

/** The links out of one note and the links into it. */
data class VaultLinks(
    val outgoing: List<VaultLink>,
    val incoming: List<VaultLink>,
)

/** What the vault screens need from the Rust core. Calls block, so run them off the main thread. */
interface VaultGateway {
    /** Notes matching [query] by title, description, tags, or text, at most [limit]. */
    fun search(
        query: String,
        limit: Int,
    ): List<VaultNote>

    /** The note at the relative [path]. */
    fun read(path: String): VaultNote

    /** Writes [source] as the whole note at [path], creating it when it does not exist. */
    fun write(
        path: String,
        source: String,
    ): VaultNote

    /** Adds [text] to the end of the note at [path]. */
    fun append(
        path: String,
        text: String,
    ): VaultNote

    /** The links out of and into the note at [path]. */
    fun links(path: String): VaultLinks

    /** Paths of the notes that no other note links to. */
    fun orphans(): List<String>

    /** The neighborhood of [start] as text, [depth] links deep, capped at [tokenBudget] bytes. */
    fun map(
        start: String,
        depth: Int,
        tokenBudget: Int,
    ): String
}

/** The real gateway over the UniFFI [VaultRuntime]. Core errors leave as [AreaFailure]. */
class CoreVaultGateway(
    private val lazyRuntime: Lazy<VaultRuntime>,
) : VaultGateway {
    private val runtime: VaultRuntime by lazyRuntime

    override fun search(
        query: String,
        limit: Int,
    ): List<VaultNote> = guard { runtime.search(query, limit.toUInt()).map { it.toNote() } }

    override fun read(path: String): VaultNote = guard { runtime.read(path).toNote() }

    override fun write(
        path: String,
        source: String,
    ): VaultNote = guard { runtime.write(path, source).toNote() }

    override fun append(
        path: String,
        text: String,
    ): VaultNote = guard { runtime.append(path, text).toNote() }

    override fun links(path: String): VaultLinks =
        guard {
            runtime.links(path).let { view ->
                VaultLinks(
                    outgoing = view.outgoing.map { it.toLink() },
                    incoming = view.incoming.map { it.toLink() },
                )
            }
        }

    override fun orphans(): List<String> = guard { runtime.orphans() }

    override fun map(
        start: String,
        depth: Int,
        tokenBudget: Int,
    ): String = guard { runtime.map(start, depth.toUInt(), tokenBudget.toUInt()) }

    private fun VaultNoteView.toNote() =
        VaultNote(
            path = path,
            title = title,
            description = description,
            noteType = noteType,
            tags = tags,
            aliases = aliases,
            updated = updated,
            headings = headings,
            source = source,
        )

    private fun VaultLinkView.toLink() =
        VaultLink(
            from = from,
            target = target,
            heading = heading,
            alias = alias,
            resolved = resolved,
            resolvedPath = resolvedPath,
        )

    private inline fun <T> guard(block: () -> T): T =
        try {
            block()
        } catch (failure: VaultRuntimeException) {
            throw AreaFailure(failure.problem(), failure)
        }
}
