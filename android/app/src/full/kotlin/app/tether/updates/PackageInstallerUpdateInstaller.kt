package app.tether.updates

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageInstaller
import android.os.Build
import android.provider.Settings
import androidx.core.content.ContextCompat
import androidx.core.net.toUri
import java.io.File
import java.io.IOException

private const val APK_NAME = "tether-update.apk"

/** Full build installer: stages the verified APK with PackageInstaller and asks Android to install it. */
internal class PackageInstallerUpdateInstaller(
    private val context: Context,
) : UpdateInstaller {
    override val installsInApp: Boolean = true

    private val packageInstaller: PackageInstaller
        get() = context.packageManager.packageInstaller

    private val resultAction = context.packageName + ".UPDATE_INSTALL_RESULT"

    override fun canRequestInstalls(): Boolean = context.packageManager.canRequestPackageInstalls()

    override fun openInstallPermissionSettings() {
        val intent =
            Intent(
                Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                ("package:" + context.packageName).toUri(),
            ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        context.startActivity(intent)
    }

    /**
     * Copies [apk] into a PackageInstaller session and commits it. Blocks while it copies the file,
     * so call it from a background dispatcher, never the main thread.
     */
    override fun install(
        apk: File,
        listener: InstallListener,
    ) {
        abandonOldSessions()
        val sessionId =
            try {
                packageInstaller.createSession(sessionParams(apk))
            } catch (e: IOException) {
                listener.onFailure(InstallFailure.Storage)
                return
            }
        packageInstaller.openSession(sessionId).use { session ->
            var committed = false
            var receiver: InstallResultReceiver? = null
            try {
                receiver = registerResultReceiver(listener)
                copyApk(session, apk)
                session.commit(resultIntent(sessionId).intentSender)
                committed = true
            } catch (e: IOException) {
                listener.onFailure(InstallFailure.Storage)
            } finally {
                if (!committed) {
                    // Any failure before the commit, not only an IOException, must stop listening and
                    // free the session; a later exception still reaches the caller.
                    receiver?.unregister()
                    session.abandon()
                }
            }
        }
    }

    override fun openStorePage() = Unit

    private fun abandonOldSessions() {
        packageInstaller.mySessions
            .filter { it.isActive }
            .forEach { packageInstaller.abandonSession(it.sessionId) }
    }

    private fun sessionParams(apk: File): PackageInstaller.SessionParams =
        PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL).apply {
            setAppPackageName(context.packageName)
            setSize(apk.length())
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED)
            }
        }

    private fun copyApk(
        session: PackageInstaller.Session,
        apk: File,
    ) {
        session.openWrite(APK_NAME, 0, apk.length()).use { out ->
            apk.inputStream().use { input -> input.copyTo(out) }
            session.fsync(out)
        }
    }

    private fun registerResultReceiver(listener: InstallListener): InstallResultReceiver {
        val receiver = InstallResultReceiver(listener)
        ContextCompat.registerReceiver(
            context,
            receiver,
            IntentFilter(resultAction),
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )
        return receiver
    }

    private fun resultIntent(sessionId: Int): PendingIntent =
        PendingIntent.getBroadcast(
            context,
            sessionId,
            Intent(resultAction).setPackage(context.packageName),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
        )

    /** Receives the status of one session. Stops listening after the final result. */
    private inner class InstallResultReceiver(
        private val listener: InstallListener,
    ) : BroadcastReceiver() {
        private var active = true

        override fun onReceive(
            context: Context,
            intent: Intent,
        ) {
            if (!active) return
            val status =
                intent.getIntExtra(
                    PackageInstaller.EXTRA_STATUS,
                    PackageInstaller.STATUS_FAILURE,
                )
            if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
                showConfirmation(intent)
                return
            }
            unregister()
            if (status == PackageInstaller.STATUS_SUCCESS) {
                listener.onSuccess()
            } else {
                listener.onFailure(installFailureFor(status) ?: InstallFailure.Other)
            }
        }

        /** Stops listening. Safe to call more than once. */
        fun unregister() {
            if (!active) return
            active = false
            context.unregisterReceiver(this)
        }

        private fun showConfirmation(intent: Intent) {
            val confirmation = confirmationIntent(intent)
            if (confirmation == null) {
                unregister()
                listener.onFailure(InstallFailure.Other)
                return
            }
            confirmation.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(confirmation)
            listener.onConfirmationShown()
        }
    }
}

private fun confirmationIntent(intent: Intent): Intent? =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        intent.getParcelableExtra(Intent.EXTRA_INTENT, Intent::class.java)
    } else {
        @Suppress("DEPRECATION")
        intent.getParcelableExtra(Intent.EXTRA_INTENT)
    }
