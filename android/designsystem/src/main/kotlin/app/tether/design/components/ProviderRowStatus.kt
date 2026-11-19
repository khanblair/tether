package app.tether.design.components

/**
 * Key status of one provider. Each status has its own icon, and [ProviderRow] adds a text state.
 */
enum class ProviderRowStatus { Unset, Ok, BadKey, NetworkError, Blocked }
