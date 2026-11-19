//! Identifier newtypes. Each wraps a ULID string created by the store.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps an identifier string. It does not check the format.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// The identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

id_type!(
    /// Identifies a project (a repository in a private workspace folder).
    ProjectId
);
id_type!(
    /// Identifies one agent session inside a project.
    SessionId
);
id_type!(
    /// Identifies one message in a session transcript.
    MessageId
);
id_type!(
    /// Identifies one tool call the agent proposed.
    ToolCallId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_round_trips_through_string_and_json() {
        let id = SessionId::new("01J0SESSION");
        assert_eq!(id.as_str(), "01J0SESSION");
        assert_eq!(id.to_string(), "01J0SESSION");
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"01J0SESSION\"");
        assert_eq!(String::from(id), "01J0SESSION");
    }
}
