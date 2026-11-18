//! Engine types converted to FFI view types.

use tether_types::CoreEvent;

/// FFI view of [`CoreEvent`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum CoreEventView {
    /// The core finished starting.
    Ready {
        /// Core version.
        version: String,
    },
    /// A test event.
    Test {
        /// Free text chosen by the caller.
        message: String,
    },
}

impl From<CoreEvent> for CoreEventView {
    fn from(event: CoreEvent) -> Self {
        match event {
            CoreEvent::Ready { version } => Self::Ready { version },
            CoreEvent::Test { message } => Self::Test { message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_converts_to_view() {
        let view = CoreEventView::from(CoreEvent::Test {
            message: "m".into(),
        });
        assert_eq!(
            view,
            CoreEventView::Test {
                message: "m".into()
            }
        );
    }
}
