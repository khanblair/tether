package app.tether.session

import android.net.Uri

/** A picked image stays local until the core has a blob reference to send. */
data class ImageDraft(
    val uri: Uri,
    val mediaType: String,
)

/** Only image MIME types can enter the attachment tray. */
internal fun isImageType(mediaType: String): Boolean =
    mediaType == "image/jpeg" || mediaType == "image/png" || mediaType == "image/webp"
