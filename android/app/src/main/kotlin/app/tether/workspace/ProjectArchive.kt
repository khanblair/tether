package app.tether.workspace

import java.io.InputStream
import java.io.OutputStream
import java.nio.file.Files
import java.nio.file.Path
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream

/** Creates a project folder under the app workspace root. */
fun createEmptyProject(root: Path, name: String): Path {
    require(name.matches(Regex("[A-Za-z0-9][A-Za-z0-9._-]*"))) { "invalid project name" }
    Files.createDirectories(root)
    return Files.createDirectory(root.resolve(name))
}

/** Unpacks a zip into a new project without allowing entries outside that folder. */
fun importProjectZip(input: InputStream, root: Path, name: String): Path {
    val destination = createEmptyProject(root, name)
    ZipInputStream(input).use { zip ->
        while (true) {
            val entry = zip.nextEntry ?: break
            val target = archiveTarget(destination, entry.name)
            if (entry.isDirectory) {
                Files.createDirectories(target)
            } else {
                Files.createDirectories(target.parent)
                Files.newOutputStream(target).use { zip.copyTo(it) }
            }
            zip.closeEntry()
        }
    }
    return destination
}

/** Writes each project file once, in sorted path order, preserving file bytes. */
fun exportProjectZip(project: Path, output: OutputStream) {
    ZipOutputStream(output).use { zip ->
        Files.walk(project).use { paths ->
            paths.filter { it != project }
                .sorted()
                .forEach { path ->
                    require(!Files.isSymbolicLink(path)) { "symbolic links are not exported" }
                    val name = project.relativize(path).toString().replace('\\', '/')
                    val entryName = if (Files.isDirectory(path)) "$name/" else name
                    zip.putNextEntry(ZipEntry(entryName))
                    if (Files.isRegularFile(path)) Files.newInputStream(path).use { it.copyTo(zip) }
                    zip.closeEntry()
                }
        }
    }
}

private fun archiveTarget(root: Path, name: String): Path {
    require(name.isNotEmpty() && !name.contains('\\') && !name.startsWith('/')) {
        "invalid archive path"
    }
    val target = root.resolve(name).normalize()
    require(target != root && target.startsWith(root)) { "archive path leaves project" }
    return target
}
