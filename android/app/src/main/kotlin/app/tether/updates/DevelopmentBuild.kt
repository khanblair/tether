package app.tether.updates

import android.content.Context
import android.content.pm.ApplicationInfo

/**
 * A debuggable build reports a version that the release channel does not match, so it must not
 * check for or offer a release. A release install over a debug build would also fail on the
 * signature, so the offer is hidden too.
 */
internal fun updatesEnabled(isDebuggable: Boolean): Boolean = !isDebuggable

/** True when this app was built as a debug build. Read once, when the update flow is created. */
internal fun isDebuggableBuild(context: Context): Boolean =
    context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
