use crate::habit_management::domain::habit::HabitError;

#[derive(Debug, Clone, PartialEq)]
pub struct HabitDescription(String);

impl HabitDescription {
    pub const MIN_LEN: usize = 1;
    pub const MAX_LEN: usize = 50;

    pub fn new(value: String) -> Result<HabitDescription, HabitError> {
        Ok(HabitDescription(value))
    }
}
