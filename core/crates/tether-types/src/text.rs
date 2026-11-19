//! Stable text forms for the enums the store and config files keep as text.

use thiserror::Error;

/// The text is not a known value of the enum it was parsed as.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown {kind} value: {value}")]
pub struct UnknownValue {
    /// The enum that was being parsed.
    pub kind: &'static str,
    /// The text that matched no variant.
    pub value: String,
}

/// Gives a field-less enum `ALL`, `as_str`, and `FromStr`, with one text form per variant.
macro_rules! text_enum {
    ($ty:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $ty {
            /// Every variant, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The stable text form kept in the database and in config files.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl ::std::str::FromStr for $ty {
            type Err = $crate::text::UnknownValue;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                match text {
                    $($text => Ok(Self::$variant),)+
                    other => Err($crate::text::UnknownValue {
                        kind: stringify!($ty),
                        value: other.to_owned(),
                    }),
                }
            }
        }
    };
}

pub(crate) use text_enum;
