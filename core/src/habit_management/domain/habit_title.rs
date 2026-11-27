use crate::habit_management::domain::habit::HabitError;

#[derive(Debug, Clone, PartialEq)]
pub struct HabitTitle(String);

impl HabitTitle {
    pub const MIN_LEN: usize = 1;
    pub const MAX_LEN: usize = 50;

    // @law: tab, newline and carriage return become a space, then the other control (char::is_control) and bidi-override (U+202A..202E, U+2066..2069) characters are stripped before trim and length, so a title made only of them is refused as TitleLength; length counts characters, not bytes
    pub fn new(value: String) -> Result<HabitTitle, HabitError> {
        let value: String = value
            .chars()
            .map(|c| {
                if matches!(c, '\t' | '\n' | '\r') {
                    ' '
                } else {
                    c
                }
            })
            .filter(|c| {
                !c.is_control() && !matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
            })
            .collect();
        Self::within_range(value)
    }

    // @law: a title stored before control characters were stripped must still load, so the codec falls back here, keeping it as stored (trimmed and length-checked only) rather than quarantining the whole snapshot
    pub fn from_stored(value: String) -> Result<HabitTitle, HabitError> {
        Self::within_range(value)
    }

    fn within_range(value: String) -> Result<HabitTitle, HabitError> {
        let value = value.trim().to_string();
        let length = value.chars().count();

        if length < Self::MIN_LEN || length > Self::MAX_LEN {
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
