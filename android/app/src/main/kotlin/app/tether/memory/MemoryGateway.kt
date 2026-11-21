package app.tether.memory

import app.tether.core.generated.MemoryActionView
import app.tether.core.generated.MemoryRuntime
import app.tether.core.generated.MemoryRuntimeException
import app.tether.core.generated.MemoryTargetView

/**
 * What the memory screen needs from the Rust core. Every call blocks, so callers run it off the
 * main thread. A fake implements this in unit tests, so no test loads the native library.
 */
interface MemoryGateway {
    /** The whole text of [file], with its entries joined by the section sign. */
    fun read(file: MemoryFile): String

    /** Applies [edit] to [file] and returns the file's text after the change. */
    fun apply(
        file: MemoryFile,
        edit: MemoryEdit,
    ): String
}

/** One change to a memory file. Each names an exact entry, so the core can refuse a stale one. */
sealed interface MemoryEdit {
    /** Adds [entry] unless the same entry is already there. */
    data class Add(
        val entry: String,
    ) : MemoryEdit

    /** Replaces the entry [old] with [new]. */
    data class Replace(
        val old: String,
        val new: String,
    ) : MemoryEdit

    /** Removes the entry [entry]. */
    data class Remove(
        val entry: String,
    ) : MemoryEdit
}

/**
 * The real gateway over the UniFFI [MemoryRuntime]. The runtime opens on the first call, off the
 * main thread. Core errors leave as [AreaFailure].
 */
class CoreMemoryGateway(
    private val lazyRuntime: Lazy<MemoryRuntime>,
) : MemoryGateway {
    private val runtime: MemoryRuntime by lazyRuntime

    override fun read(file: MemoryFile): String = guard { runtime.read(file.coreTarget()) }

    override fun apply(
        file: MemoryFile,
        edit: MemoryEdit,
    ): String = guard { runtime.apply(file.coreTarget(), edit.coreAction()).content }

    private fun MemoryFile.coreTarget(): MemoryTargetView =
        when (this) {
            MemoryFile.Memory -> MemoryTargetView.MEMORY
            MemoryFile.User -> MemoryTargetView.USER
        }

    private fun MemoryEdit.coreAction(): MemoryActionView =
        when (this) {
            is MemoryEdit.Add -> MemoryActionView.Add(entry)
            is MemoryEdit.Replace -> MemoryActionView.Replace(old, new)
            is MemoryEdit.Remove -> MemoryActionView.Remove(entry)
        }

    private inline fun <T> guard(block: () -> T): T =
        try {
            block()
        } catch (failure: MemoryRuntimeException) {
            throw AreaFailure(failure.problem(), failure)
        }
}
