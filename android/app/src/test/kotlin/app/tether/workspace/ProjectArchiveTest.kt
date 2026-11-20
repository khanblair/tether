package app.tether.workspace

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.nio.file.Files
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class ProjectArchiveTest {
    @get:Rule val temporary = TemporaryFolder()

    @Test fun zipRoundTripKeepsEveryFileByte() {
        val root = temporary.root.toPath()
        val original = createEmptyProject(root, "original")
        Files.createDirectories(original.resolve("nested"))
        val bytes = byteArrayOf(0, 1, 2, -1, 10)
        Files.write(original.resolve("nested/file.bin"), bytes)
        Files.write(original.resolve("empty.txt"), byteArrayOf())
        val archive = ByteArrayOutputStream()
        exportProjectZip(original, archive)

        val imported = importProjectZip(ByteArrayInputStream(archive.toByteArray()), root, "copy")
        assertArrayEquals(bytes, Files.readAllBytes(imported.resolve("nested/file.bin")))
        assertArrayEquals(byteArrayOf(), Files.readAllBytes(imported.resolve("empty.txt")))
    }

    @Test fun zipEntryCannotEscapeTheNewProject() {
        val bytes = ByteArrayOutputStream()
        java.util.zip.ZipOutputStream(bytes).use { zip ->
            zip.putNextEntry(java.util.zip.ZipEntry("../outside.txt"))
            zip.write(1)
            zip.closeEntry()
        }
        val root = temporary.root.toPath()
        val failed = runCatching {
            importProjectZip(ByteArrayInputStream(bytes.toByteArray()), root, "project")
        }.isFailure
        assertTrue(failed)
        assertFalse(Files.exists(root.resolve("outside.txt")))
    }
}
