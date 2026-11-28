package app.tether.memory

/** A memory gateway that keeps both files in memory and applies the same rules as the core. */
class FakeMemoryGateway(
    initial: Map<MemoryFile, String> = emptyMap(),
) : MemoryGateway {
    val files = initial.toMutableMap()

    /** Throws this from the next read, then clears itself. */
    var readFailure: Exception? = null

    override fun read(file: MemoryFile): String {
        readFailure?.let {
            readFailure = null
            throw it
        }
        return files[file].orEmpty()
    }

    override fun apply(
        file: MemoryFile,
        edit: MemoryEdit,
    ): String {
        val entries = splitEntries(files[file].orEmpty()).toMutableList()
        when (edit) {
            is MemoryEdit.Add -> if (edit.entry !in entries) entries += edit.entry
            is MemoryEdit.Replace -> {
                val slot = entries.indexOf(edit.old)
                if (slot < 0) throw AreaFailure(AreaProblem.NotFound)
                entries[slot] = edit.new
            }
            is MemoryEdit.Remove ->
                if (!entries.remove(
                        edit.entry,
                    )
                ) {
                    throw AreaFailure(AreaProblem.NotFound)
                }
        }
        val updated = entries.joinToString(SECTION)
        if (characterCount(updated) > file.cap) throw AreaFailure(AreaProblem.OverCap)
        files[file] = updated
        return updated
    }

    private companion object {
        const val SECTION = "\n§\n"
    }
}

/** A skills gateway over a fixed list of skills. */
class FakeSkillsGateway(
    val items: List<SkillItem> = emptyList(),
    val details: Map<String, SkillDetail> = emptyMap(),
    val sampleItems: MutableList<SampleSkillItem> = mutableListOf(),
    val notice: String = "",
) : SkillsGateway {
    /** Names passed to a successful [installSample], in order. */
    val installs = mutableListOf<String>()

    /** Throws this from the next call of any kind, then clears itself. */
    var failure: Exception? = null

    private fun failIfSet() {
        failure?.let {
            failure = null
            throw it
        }
    }

    override fun list(): List<SkillItem> {
        failIfSet()
        return items
    }

    override fun read(folder: String): SkillDetail {
        failIfSet()
        return details[folder] ?: throw AreaFailure(AreaProblem.NotFound)
    }

    override fun samples(): List<SampleSkillItem> {
        failIfSet()
        return sampleItems.toList()
    }

    override fun installSample(name: String) {
        failIfSet()
        val slot = sampleItems.indexOfFirst { it.name == name }
        if (slot < 0) throw AreaFailure(AreaProblem.NotFound)
        installs += name
        sampleItems[slot] = sampleItems[slot].copy(installed = true)
    }

    override fun samplesNotice(): String {
        failIfSet()
        return notice
    }
}

/** A vault gateway with notes and links that a test sets up. Writes are recorded in [writes]. */
class FakeVaultGateway(
    val notes: MutableMap<String, VaultNote> = mutableMapOf(),
    val links: Map<String, VaultLinks> = emptyMap(),
    val orphanPaths: List<String> = emptyList(),
    val mapText: String = "",
) : VaultGateway {
    val writes = mutableListOf<String>()

    /** Throws this from the next call of any kind, then clears itself. */
    var failure: Exception? = null

    private fun failIfSet() {
        failure?.let {
            failure = null
            throw it
        }
    }

    override fun search(
        query: String,
        limit: Int,
    ): List<VaultNote> {
        failIfSet()
        return notes.values.filter { it.title.contains(query, ignoreCase = true) }.take(limit)
    }

    override fun read(path: String): VaultNote {
        failIfSet()
        return notes[path] ?: throw AreaFailure(AreaProblem.NotFound)
    }

    override fun write(
        path: String,
        source: String,
    ): VaultNote {
        failIfSet()
        writes += "write:$path"
        return note(path, source).also { notes[path] = it }
    }

    override fun append(
        path: String,
        text: String,
    ): VaultNote {
        failIfSet()
        writes += "append:$path:$text"
        val existing = notes[path] ?: throw AreaFailure(AreaProblem.NotFound)
        return note(path, existing.source + text).also { notes[path] = it }
    }

    override fun links(path: String): VaultLinks {
        failIfSet()
        return links[path] ?: throw AreaFailure(AreaProblem.NotFound)
    }

    override fun orphans(): List<String> {
        failIfSet()
        return orphanPaths
    }

    override fun map(
        start: String,
        depth: Int,
        tokenBudget: Int,
    ): String {
        failIfSet()
        return mapText
    }
}

/** A note with a title taken from its path and a description of [description]. */
fun note(
    path: String,
    source: String,
    description: String = "A note",
): VaultNote =
    VaultNote(
        path = path,
        title = path.substringAfterLast('/').removeSuffix(".md"),
        description = description,
        noteType = null,
        tags = emptyList(),
        aliases = emptyList(),
        updated = null,
        headings = emptyList(),
        source = source,
    )

/** A skill that parsed, with [name] as its name and folder. */
fun skill(
    folder: String,
    description: String = "Does $folder",
): SkillItem =
    SkillItem(folder = folder, name = folder, description = description, valid = true, error = null)
