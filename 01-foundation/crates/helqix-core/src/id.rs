//! Validated, deterministic identifiers for HELQIX Core.
//!
//! An [`Identifier`] is the name of a definition (a component, property,
//! interface, behaviour, constraint). It is a pure value type: no randomness,
//! no global registry, no clock, no I/O. Parsing the same text always yields the
//! same result, so identifiers are safe to use as stable map keys and in
//! deterministic serialized output.
//!
//! # Grammar
//!
//! ```text
//! identifier = start *continue
//! start      = ASCII letter                      ; A-Z a-z
//! continue   = ASCII letter / ASCII digit / "_"  ; A-Z a-z 0-9 _
//! ```
//!
//! - Length is 1 to [`MAX_IDENTIFIER_LENGTH`] characters (inclusive).
//! - Matching is case-sensitive: `Motor` and `motor` are different identifiers.
//! - No normalization is applied. Input is accepted exactly as written or
//!   rejected; nothing is trimmed, lowercased or rewritten.
//!
//! # Validation order
//!
//! Failures are reported in a fixed order so the same bad input always yields
//! the same error: empty, then too long, then invalid start, then the first
//! invalid character. A 100-character string beginning with a digit therefore
//! reports [`IdentifierError::TooLong`], not `InvalidStart`.
//!
//! All lengths and positions are counted in Unicode scalar values (`char`s),
//! never bytes, and positions are zero-based.

use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use crate::error::IdentifierError;

/// Maximum number of characters in an [`Identifier`].
pub const MAX_IDENTIFIER_LENGTH: usize = 64;

/// A validated identifier.
///
/// The only way to obtain one is through validation ([`Identifier::parse`],
/// [`FromStr`], or `TryFrom`), so holding an `Identifier` is proof the text
/// satisfies the grammar in the module documentation.
///
/// Ordering is byte-wise lexicographic on the text, which is deterministic and
/// suitable for `BTreeMap`/`BTreeSet` keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier(String);

impl Identifier {
    /// Validates `text` and returns an identifier that owns a copy of it.
    ///
    /// # Errors
    ///
    /// Returns the first [`IdentifierError`] found, in the order described in
    /// the module documentation.
    pub fn parse(text: &str) -> Result<Self, IdentifierError> {
        validate(text)?;
        Ok(Self(text.to_owned()))
    }

    /// Returns the identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the identifier and returns its text.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

/// Checks `text` against the identifier grammar.
fn validate(text: &str) -> Result<(), IdentifierError> {
    let Some(first) = text.chars().next() else {
        return Err(IdentifierError::Empty);
    };

    let length = text.chars().count();
    if length > MAX_IDENTIFIER_LENGTH {
        return Err(IdentifierError::TooLong {
            length,
            max: MAX_IDENTIFIER_LENGTH,
        });
    }

    if !first.is_ascii_alphabetic() {
        return Err(IdentifierError::InvalidStart { character: first });
    }

    for (position, character) in text.chars().enumerate().skip(1) {
        if !(character.is_ascii_alphanumeric() || character == '_') {
            return Err(IdentifierError::InvalidCharacter {
                character,
                position,
            });
        }
    }

    Ok(())
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Lets `HashMap<Identifier, _>` and `BTreeMap<Identifier, _>` be queried with
/// a plain `&str`. Sound because the derived `Hash`/`Eq`/`Ord` on the inner
/// `String` agree with those of `str`.
impl Borrow<str> for Identifier {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl FromStr for Identifier {
    type Err = IdentifierError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl TryFrom<&str> for Identifier {
    type Error = IdentifierError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::parse(text)
    }
}

impl TryFrom<String> for Identifier {
    type Error = IdentifierError;

    /// Validates `text` and reuses its allocation.
    fn try_from(text: String) -> Result<Self, Self::Error> {
        validate(&text)?;
        Ok(Self(text))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use super::*;
    use crate::error::{CoreError, CoreResult};

    fn parse_err(text: &str) -> IdentifierError {
        Identifier::parse(text).expect_err("expected parse failure")
    }

    #[test]
    fn accepts_valid_identifiers() {
        for text in ["a", "Z", "motor", "Motor_2", "a_b_c", "x1", "A9_z"] {
            let id = Identifier::parse(text).expect(text);
            assert_eq!(id.as_str(), text);
        }
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(parse_err(""), IdentifierError::Empty);
    }

    #[test]
    fn length_boundary_is_inclusive() {
        let at_limit = "a".repeat(MAX_IDENTIFIER_LENGTH);
        assert!(Identifier::parse(&at_limit).is_ok());

        let over_limit = "a".repeat(MAX_IDENTIFIER_LENGTH + 1);
        assert_eq!(
            parse_err(&over_limit),
            IdentifierError::TooLong {
                length: MAX_IDENTIFIER_LENGTH + 1,
                max: MAX_IDENTIFIER_LENGTH,
            }
        );
    }

    #[test]
    fn length_counts_characters_not_bytes() {
        // 'é' is 2 bytes but 1 char: 1 + 64 chars = 65 -> TooLong, and it is
        // reported before the invalid character is reached.
        let text = format!("a{}", "é".repeat(MAX_IDENTIFIER_LENGTH));
        assert_eq!(
            parse_err(&text),
            IdentifierError::TooLong {
                length: MAX_IDENTIFIER_LENGTH + 1,
                max: MAX_IDENTIFIER_LENGTH,
            }
        );
    }

    #[test]
    fn rejects_invalid_start() {
        assert_eq!(
            parse_err("9motor"),
            IdentifierError::InvalidStart { character: '9' }
        );
        assert_eq!(
            parse_err("_motor"),
            IdentifierError::InvalidStart { character: '_' }
        );
        assert_eq!(
            parse_err("-motor"),
            IdentifierError::InvalidStart { character: '-' }
        );
        assert_eq!(
            parse_err("émotor"),
            IdentifierError::InvalidStart { character: 'é' }
        );
    }

    #[test]
    fn reports_first_invalid_character_with_char_position() {
        assert_eq!(
            parse_err("ab-c-d"),
            IdentifierError::InvalidCharacter {
                character: '-',
                position: 2,
            }
        );
        assert_eq!(
            parse_err("a b"),
            IdentifierError::InvalidCharacter {
                character: ' ',
                position: 1,
            }
        );
        // 'é' occupies bytes 1..3, but its char position is 1.
        assert_eq!(
            parse_err("aéb"),
            IdentifierError::InvalidCharacter {
                character: 'é',
                position: 1,
            }
        );
    }

    #[test]
    fn rejects_surrounding_whitespace_without_trimming() {
        assert_eq!(
            parse_err(" motor"),
            IdentifierError::InvalidStart { character: ' ' }
        );
        assert_eq!(
            parse_err("motor "),
            IdentifierError::InvalidCharacter {
                character: ' ',
                position: 5,
            }
        );
    }

    #[test]
    fn validation_order_is_fixed() {
        // Too long beats invalid start.
        let long_bad_start = format!("9{}", "a".repeat(MAX_IDENTIFIER_LENGTH));
        assert!(matches!(
            parse_err(&long_bad_start),
            IdentifierError::TooLong { .. }
        ));
        // Invalid start beats invalid character later in the text.
        assert!(matches!(
            parse_err("9-"),
            IdentifierError::InvalidStart { character: '9' }
        ));
    }

    #[test]
    fn is_case_sensitive_and_never_normalizes() {
        let upper = Identifier::parse("Motor").unwrap();
        let lower = Identifier::parse("motor").unwrap();
        assert_ne!(upper, lower);
        assert_eq!(upper.as_str(), "Motor");
    }

    #[test]
    fn parsing_is_deterministic() {
        assert_eq!(Identifier::parse("pump_1"), Identifier::parse("pump_1"));
        assert_eq!(parse_err("1pump"), parse_err("1pump"));
    }

    #[test]
    fn all_construction_paths_agree() {
        let parsed = Identifier::parse("pump_1").unwrap();
        let from_str: Identifier = "pump_1".parse().unwrap();
        let from_ref = Identifier::try_from("pump_1").unwrap();
        let from_owned = Identifier::try_from(String::from("pump_1")).unwrap();
        assert_eq!(parsed, from_str);
        assert_eq!(parsed, from_ref);
        assert_eq!(parsed, from_owned);

        assert_eq!(
            Identifier::try_from(String::from("1pump")).unwrap_err(),
            IdentifierError::InvalidStart { character: '1' }
        );
        assert_eq!(
            "1pump".parse::<Identifier>().unwrap_err(),
            IdentifierError::InvalidStart { character: '1' }
        );
    }

    #[test]
    fn display_round_trips() {
        let id = Identifier::parse("sensor_A").unwrap();
        assert_eq!(id.to_string(), "sensor_A");
        assert_eq!(Identifier::parse(&id.to_string()).unwrap(), id);
        assert_eq!(id.as_ref(), "sensor_A");
        assert_eq!(id.into_string(), "sensor_A");
    }

    #[test]
    fn ordering_is_bytewise_lexicographic() {
        let set: BTreeSet<Identifier> = ["b", "a", "B", "a1"]
            .iter()
            .map(|s| Identifier::parse(s).unwrap())
            .collect();
        let ordered: Vec<&str> = set.iter().map(Identifier::as_str).collect();
        assert_eq!(ordered, ["B", "a", "a1", "b"]);
    }

    #[test]
    fn hash_collections_can_be_queried_with_str() {
        let set: HashSet<Identifier> = [Identifier::parse("motor").unwrap()].into_iter().collect();
        assert!(set.contains("motor"));
        assert!(!set.contains("Motor"));
    }

    #[test]
    fn identifier_error_propagates_into_core_error_with_question_mark() {
        fn load(text: &str) -> CoreResult<Identifier> {
            Ok(Identifier::parse(text)?)
        }

        assert_eq!(
            load("a-b").unwrap_err(),
            CoreError::Identifier(IdentifierError::InvalidCharacter {
                character: '-',
                position: 1,
            })
        );
    }
}