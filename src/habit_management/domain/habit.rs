use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;
use crate::habit_management::domain::initial_duration::InitialDuration;
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct Habit {
    id: HabitId,
    title: HabitTitle,
    initial_duration: InitialDuration,
}

#[derive(Debug, PartialEq)]
pub enum HabitError {
    DurationTooLong { max: u32 },
    TitleLength { min: usize, max: usize },
}

impl fmt::Display for HabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HabitError::DurationTooLong { max } => {
                write!(f, "an easy habit must last no more than {max} minutes")
            }
            HabitError::TitleLength { min, max } => {
                write!(
                    f,
                    "a title size must be between {min} and {max} characters"
                )
            }
        }
    }
}

impl Error for HabitError {}

impl Habit {
    pub fn new(id: HabitId, title: HabitTitle, initial_duration: InitialDuration) -> Habit {
        Habit {
            id,
            title,
            initial_duration,
        }
    }
}
