use crate::habit_management::domain::habit::HabitError;

#[derive(Debug, Clone, PartialEq)]
pub struct HabitId(String);

impl HabitId {
    pub const MIN_LEN: usize = 1;
    pub const MAX_LEN: usize = 64;

    pub fn new(value: &str) -> Result<HabitId, HabitError> {
        Ok(HabitId(value.to_string()))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
impl From<&str> for HabitId {
    fn from(value: &str) -> HabitId {
        HabitId(value.to_string())
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

    #[test]
    fn new_accepts_values_within_the_length_bound() {
        let cases = vec!["h".repeat(HabitId::MIN_LEN), "h".repeat(HabitId::MAX_LEN)];

        for value in cases {
            assert!(HabitId::new(&value).is_ok());
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
                })
            );
        }
    }

    #[test]
    fn new_does_not_trim_surrounding_whitespace() {
        assert_ne!(HabitId::new(" h-1 "), HabitId::new("h-1"));
    }
}
