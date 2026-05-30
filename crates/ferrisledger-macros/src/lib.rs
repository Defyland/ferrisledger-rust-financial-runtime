//! Small declarative macros used by FerrisLedger crates.

/// Defines a validated string identifier newtype.
///
/// The macro is intentionally declarative instead of procedural because the
/// project only needs predictable boilerplate for domain IDs. A procedural
/// macro would add compile-time complexity without improving the runtime.
#[macro_export]
macro_rules! validated_string_id {
    ($(#[$meta:meta])* $vis:vis struct $name:ident;) => {
        $(#[$meta])*
        #[derive(
            Clone,
            Debug,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            serde::Deserialize,
            serde::Serialize,
        )]
        #[serde(transparent)]
        $vis struct $name(String);

        impl $name {
            /// Creates a new validated identifier.
            ///
            /// Identifiers are short, non-empty ASCII tokens. That keeps them
            /// log-safe and URL-safe while still allowing UUIDs and prefixed
            /// business identifiers.
            pub fn new(value: impl Into<String>) -> Result<Self, String> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(format!("{} cannot be empty", stringify!($name)));
                }
                if value.len() > 128 {
                    return Err(format!("{} is longer than 128 bytes", stringify!($name)));
                }
                if !value.is_ascii() || value.chars().any(char::is_whitespace) {
                    return Err(format!(
                        "{} must be ASCII and must not contain whitespace",
                        stringify!($name)
                    ));
                }
                Ok(Self(value))
            }

            /// Returns the identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = String;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }
    };
}
