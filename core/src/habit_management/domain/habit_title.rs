use crate::habit_management::domain::habit::HabitError;

#[derive(Debug, Clone, PartialEq)]
pub struct HabitTitle(String);

impl HabitTitle {
    pub const MIN_LEN: usize = 1;
    pub const MAX_LEN: usize = 50;

    pub fn new(value: String) -> Result<HabitTitle, HabitError> {
        let value = value.trim().to_string();

        if value.len() < Self::MIN_LEN || value.len() > Self::MAX_LEN {
            return Err(HabitError::TitleLength {
                min: Self::MIN_LEN,
                max: Self::MAX_LEN,
            });
        }

        Ok(HabitTitle(value))
    }

    pub fn matches(&self, other: &HabitTitle) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_leading_and_trailing_whitespace() {
        let trimmed = HabitTitle::new(String::from("Read one page")).unwrap();
        let padded = HabitTitle::new(String::from("  Read one page  ")).unwrap();

        assert_eq!(trimmed, padded);
    }

    #[test]
    fn new_rejects_a_title_that_is_blank_after_trimming() {
        let result = HabitTitle::new(String::from("   "));

        assert_eq!(
            result,
            Err(HabitError::TitleLength {
                min: HabitTitle::MIN_LEN,
                max: HabitTitle::MAX_LEN,
            })
        );
    }

    #[test]
    fn new_accepts_a_title_that_is_within_range_only_after_trimming() {
        let value = format!("{} ", "a".repeat(HabitTitle::MAX_LEN));

        let result = HabitTitle::new(value);

        assert!(result.is_ok());
    }

    #[test]
    fn matches_is_case_insensitive_while_partial_eq_stays_strict() {
        let lowercase = HabitTitle::new(String::from("lire une page")).unwrap();
        let capitalized = HabitTitle::new(String::from("Lire une page")).unwrap();

        assert!(lowercase.matches(&capitalized));
        assert_ne!(lowercase, capitalized);
    }

    #[test]
    fn matches_is_case_insensitive_for_non_ascii_titles() {
        let lowercase = HabitTitle::new(String::from("étirements")).unwrap();
        let uppercase = HabitTitle::new(String::from("ÉTIREMENTS")).unwrap();

        assert!(lowercase.matches(&uppercase));
    }

    #[test]
    fn matches_returns_false_for_different_titles() {
        let a = HabitTitle::new(String::from("Lire une page")).unwrap();
        let b = HabitTitle::new(String::from("Bouger un peu")).unwrap();

        assert!(!a.matches(&b));
    }
}
