package app.tether.session

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ImageDraftTest {
    @Test fun supportedImageTypesEnterTheTray() {
        assertTrue(isImageType("image/png"))
        assertTrue(isImageType("image/jpeg"))
        assertTrue(isImageType("image/webp"))
    }

    @Test fun arbitraryContentCannotEnterTheTray() {
        assertFalse(isImageType("text/plain"))
        assertFalse(isImageType("image/svg+xml"))
    }
}
