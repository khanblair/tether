package app.tether.preview

import app.tether.core.generated.FilesRuntimeException
import app.tether.files.FilesProblem
import app.tether.files.problem
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CorePreviewFilesGatewayTest {
    @Test fun aNegativeLimitReadsNothing() {
        assertEquals(0UL, previewLimit(-5L))
    }

    @Test fun aLimitPassesThroughUnchanged() {
        assertEquals(10_485_760UL, previewLimit(10L * 1024L * 1024L))
    }

    @Test fun aHugeSizeIsCappedAtLongMax() {
        assertEquals(Long.MAX_VALUE, previewSize(ULong.MAX_VALUE))
        assertEquals(42L, previewSize(42UL))
    }

    @Test fun aTruncatedFileKeepsItsFullSize() {
        val file = fileBytesOf(byteArrayOf(1, 2, 3), 10UL, truncated = true)
        assertArrayEquals(byteArrayOf(1, 2, 3), file.bytes)
        assertEquals(10L, file.sizeBytes)
        assertTrue(file.truncated)
    }

    @Test fun anEmptyFileHasNoBytesAndNoTruncation() {
        val file = fileBytesOf(ByteArray(0), 0UL, truncated = false)
        assertArrayEquals(ByteArray(0), file.bytes)
        assertEquals(0L, file.sizeBytes)
        assertFalse(file.truncated)
    }

    @Test fun aMissingFileMapsToNotFound() {
        assertEquals(FilesProblem.NotFound, FilesRuntimeException.NotFound().problem())
    }

    @Test fun anUnsafePathMapsToInvalidPath() {
        assertEquals(
            FilesProblem.InvalidPath,
            FilesRuntimeException.InvalidPath().problem(),
        )
    }

    @Test fun anUnreadableWorkspaceMapsToUnavailable() {
        assertEquals(
            FilesProblem.Unavailable,
            FilesRuntimeException.Unavailable().problem(),
        )
    }
}
