use crate::habit_management::domain::habit::HabitError;

#[derive(Debug, Clone, PartialEq)]
pub struct InitialDuration(u32);

impl InitialDuration {
    pub const MAX: u32 = 5;

    pub fn new(value: u32) -> Result<InitialDuration, HabitError> {
        Ok(InitialDuration(value))
    }
}
