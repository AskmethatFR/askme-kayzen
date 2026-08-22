use crate::habit_management::domain::habit::HabitError;

#[derive(Debug, Clone, PartialEq)]
pub struct HabitId(String);

impl HabitId {
    pub const MIN_LEN: usize = 1;
    pub const MAX_LEN: usize = 64;

    /// Retry 2 on #34 (adr-0010 trigger #7): once a habit can be restored
    /// from a file or `localStorage`, an id is user-supplied, not
    /// "already validated" — it is arbitrary text that happens to pass
    /// length. Restricted to ASCII alphanumeric plus dash: exactly what
    /// `UuidGenerator` emits (`Uuid::new_v4().to_string()` is lowercase hex
    /// and dashes only), so no legitimate id is excluded, while a path
    /// separator, URL delimiter, or whitespace/control character can no
    /// longer reach `Route::HabitDetail { id }` as a URL path segment.
    pub fn new(value: &str) -> Result<HabitId, HabitError> {
        if value.len() < Self::MIN_LEN || value.len() > Self::MAX_LEN {
            return Err(HabitError::IdLength {
                min: Self::MIN_LEN,
                max: Self::MAX_LEN,
            });
        }

        if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(HabitError::IdCharset);
        }

        Ok(HabitId(value.to_string()))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    // Test List — HabitId::new boundary (adr-0009: comparison mutants on
    // MIN_LEN/MAX_LEN die only on exact-boundary tests):
    // - values within [MIN_LEN, MAX_LEN] are accepted.
    // - values outside that range are rejected with the exact error variant.
    // - no trim (Q2): a value padded with whitespace is a different id.
    //
    // Test List — HabitId::new charset (retry 2 on #34, adr-0010 trigger #7:
    // a restored id is user-supplied, not "already validated"):
    // - every character in the allowed set (upper/lower ASCII letters,
    //   digits, dash) is accepted.
    // - a value containing one disallowed character is rejected with the
    //   exact error variant, one case per dangerous class: a path separator,
    //   a URL delimiter, a whitespace/control character.

    #[test]
    fn new_accepts_values_within_the_length_bound() {
        let cases = vec!["h".repeat(HabitId::MIN_LEN), "h".repeat(HabitId::MAX_LEN)];

        for value in cases {
            assert!(
                HabitId::new(&value).is_ok(),
                "expected {value:?} (len {}) to be accepted",
                value.len()
            );
        }
    }

    #[test]
    fn new_rejects_values_outside_the_length_bound() {
        let cases = vec![String::new(), "h".repeat(HabitId::MAX_LEN + 1)];

        for value in cases {
            assert_eq!(
                HabitId::new(&value),
                Err(HabitError::IdLength {
                    min: HabitId::MIN_LEN,
                    max: HabitId::MAX_LEN,
                }),
                "expected {value:?} (len {}) to be rejected",
                value.len()
            );
        }
    }

    #[test]
    fn new_does_not_trim_surrounding_whitespace() {
        assert_ne!(HabitId::new(" h-1 "), HabitId::new("h-1"));
    }

    #[test]
    fn new_accepts_every_character_in_the_allowed_set() {
        let value = "abcXYZ019-";

        assert!(
            HabitId::new(value).is_ok(),
            "expected {value:?} to be accepted"
        );
    }

    #[test]
    fn new_rejects_a_value_containing_a_disallowed_character() {
        let cases = vec!["h/1", "h?1", "h#1", "h 1", "h\t1", "h\x011"];

        for value in cases {
            assert_eq!(
                HabitId::new(value),
                Err(HabitError::IdCharset),
                "expected {value:?} to be rejected"
            );
        }
    }
}
