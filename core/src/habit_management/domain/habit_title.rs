use crate::habit_management::domain::habit::HabitError;

#[derive(Debug, Clone, PartialEq)]
pub struct HabitTitle(String);

impl HabitTitle {
    pub const MIN_LEN: usize = 1;
    pub const MAX_LEN: usize = 50;

    // @law: tab, newline and carriage return become a space, then the other control (char::is_control), bidi-override (U+202A..202E, U+2066..2069) and invisible (U+200B, U+200E/200F, U+2060..2064, U+FEFF, U+00AD, U+3164, U+115F, U+1160) characters are stripped before trim and length, so a title made only of them is refused as TitleLength; ZWJ/ZWNJ (U+200D/200C) are kept for emoji and Persian/Indic text, but a title with nothing else left is refused too; length counts characters, not bytes
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
                !c.is_control()
                    && !matches!(
                        c,
                        '\u{202A}'..='\u{202E}'
                            | '\u{2066}'..='\u{2069}'
                            | '\u{200B}'
                            | '\u{200E}'..='\u{200F}'
                            | '\u{2060}'..='\u{2064}'
                            | '\u{FEFF}'
                            | '\u{00AD}'
                            | '\u{3164}'
                            | '\u{115F}'
                            | '\u{1160}'
                    )
            })
            .collect();
        if value
            .chars()
            .all(|c| c.is_whitespace() || matches!(c, '\u{200C}' | '\u{200D}'))
        {
            return Err(Self::length_error());
        }
        Self::within_range(value)
    }

    // @law: a title stored before control characters were stripped must still load, so the codec falls back here, keeping it as stored (trimmed and length-checked only) rather than quarantining the whole snapshot
    pub(crate) fn from_stored(value: String) -> Result<HabitTitle, HabitError> {
        Self::within_range(value)
    }

    fn length_error() -> HabitError {
        HabitError::TitleLength {
            min: Self::MIN_LEN,
            max: Self::MAX_LEN,
        }
    }

    fn within_range(value: String) -> Result<HabitTitle, HabitError> {
        let value = value.trim().to_string();

        if !(Self::MIN_LEN..=Self::MAX_LEN).contains(&value.chars().count()) {
            return Err(Self::length_error());
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
