//! Events the core sends to the platform layer.

/// One event emitted by the core. Changing this enum changes the FFI surface;
/// regenerate the Kotlin bindings with `just bindings`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreEvent {
    /// The core finished starting and reports its version.
    Ready {
        /// Core version, equal to the app version.
        version: String,
    },
    /// A test event used by P0 to prove a Rust event reaches a Compose screen.
    Test {
        /// Free text chosen by the caller.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::CoreEvent;

    #[test]
    fn events_compare_by_value() {
        let a = CoreEvent::Test {
            message: "hi".into(),
        };
        assert_eq!(
            a,
            CoreEvent::Test {
                message: "hi".into()
            }
        );
        assert_ne!(
            a,
            CoreEvent::Ready {
                version: "0.0.1".into()
            }
        );
    }
}
