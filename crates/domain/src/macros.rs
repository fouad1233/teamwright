//! Macros shared by the modules of this crate.

/// Gives an enum a stable text form and a way to read it back.
///
/// The text form is what storage keeps and what other programs see, so it
/// must never change once it has been released. Renaming a variant in code is
/// safe; changing its text is not.
macro_rules! text_enum {
    ($name:ident, kind = $kind:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            /// Every value, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The stable text form of this value.
            #[must_use]
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = $crate::error::DomainError;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                match text {
                    $($text => Ok(Self::$variant),)+
                    other => Err($crate::error::DomainError::UnknownValue {
                        kind: $kind,
                        value: other.to_owned(),
                    }),
                }
            }
        }
    };
}

pub(crate) use text_enum;
