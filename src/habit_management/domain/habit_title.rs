use crate::habit_management::domain::habit::HabitError;

#[derive(Debug, Clone, PartialEq)]
pub struct HabitTitle(String);

impl HabitTitle {
    pub const MIN_LEN: usize = 1;
    pub const MAX_LEN: usize = 50;

    pub fn new(value: String) -> Result<HabitTitle, HabitError> {
        if value.len() < Self::MIN_LEN || value.len() > Self::MAX_LEN {
            return Err(HabitError::TitleLength {
                min: Self::MIN_LEN,
                max: Self::MAX_LEN,
            });
        }

        Ok(HabitTitle(value))
    }
}
