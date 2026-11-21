package app.tether.session

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.flow.distinctUntilChanged

/** Whether the phone can reach the internet. A fake implements this in unit tests. */
interface ConnectivitySource {
    /** True while the default network has internet access. The first value is the current state. */
    val online: Flow<Boolean>
}

/** Reads the default network through ConnectivityManager callbacks. Needs ACCESS_NETWORK_STATE. */
class AndroidConnectivitySource(
    context: Context,
) : ConnectivitySource {
    private val manager = context.getSystemService(ConnectivityManager::class.java)

    override val online: Flow<Boolean> =
        callbackFlow {
            fun publish() {
                trySend(hasInternet())
            }
            val callback =
                object : ConnectivityManager.NetworkCallback() {
                    override fun onAvailable(network: Network) = publish()

                    override fun onLost(network: Network) = publish()

                    override fun onCapabilitiesChanged(
                        network: Network,
                        capabilities: NetworkCapabilities,
                    ) = publish()
                }
            publish()
            manager.registerDefaultNetworkCallback(callback)
            awaitClose { manager.unregisterNetworkCallback(callback) }
        }.distinctUntilChanged()

    /** Reads the active network at call time, so a lost network never reads as online. */
    private fun hasInternet(): Boolean {
        val capabilities = manager.getNetworkCapabilities(manager.activeNetwork) ?: return false
        return capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
    }
}
