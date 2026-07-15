use crate::habit_management::domain::habit_id::HabitId;
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct Habit {
    id: HabitId,
    description: String,
    initial_duration: u32,
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
    pub const MAX_INITIAL_DURATION: u32 = 5;
    pub const MIN_DESCRIPTION_LEN: usize = 1;
    pub const MAX_DESCRIPTION_LEN: usize = 50;

    pub fn new(
        id: String,
        description: String,
        initial_duration: u32,
    ) -> Result<Habit, HabitError> {
        if initial_duration > Self::MAX_INITIAL_DURATION {
            return Err(HabitError::DurationTooLong {
                max: Self::MAX_INITIAL_DURATION,
            });
        }

        if description.len() < Self::MIN_DESCRIPTION_LEN
            || description.len() > Self::MAX_DESCRIPTION_LEN
        {
            return Err(HabitError::DescriptionLength {
                min: Self::MIN_DESCRIPTION_LEN,
                max: Self::MAX_DESCRIPTION_LEN,
            });
        }

        let habit_id = HabitId::new(id);
        Ok(Habit {
            id: habit_id,
            description,
            initial_duration,
        })
    }
}
