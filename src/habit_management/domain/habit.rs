use crate::habit_management::domain::habit_description::HabitDescription;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::initial_duration::InitialDuration;
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct Habit {
    id: HabitId,
    description: HabitDescription,
    initial_duration: InitialDuration,
}

#[derive(Debug, PartialEq)]
pub enum HabitError {
    DurationTooLong { max: u32 },
    DescriptionLength { min: usize, max: usize },
}

impl fmt::Display for HabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HabitError::DurationTooLong { max } => {
                write!(f, "an easy habit must last no more than {max} minutes")
            }
            HabitError::DescriptionLength { min, max } => {
                write!(
                    f,
                    "a description size must be between {min} and {max} characters"
                )
            }
        }
    }
}

impl Error for HabitError {}

impl Habit {
    pub fn new(id: HabitId, description: HabitDescription, initial_duration: InitialDuration) -> Habit {
        Habit {
            id,
            description,
            initial_duration,
        }
    }
}
