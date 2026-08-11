use crate::habit_management::domain::goal::Goal;
use crate::habit_management::domain::habit::HabitError;
use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
struct BoardEntry {
    id: HabitId,
    title: HabitTitle,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct HabitBoard {
    requests: Vec<BoardEntry>,
}

#[derive(Debug, PartialEq)]
pub enum HabitBoardError {
    InvalidHabit(HabitError),
    DuplicateHabit,
    BoardFull { max: usize },
}

impl fmt::Display for HabitBoardError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HabitBoardError::InvalidHabit(error) => write!(f, "{error}"),
            HabitBoardError::DuplicateHabit => {
                write!(f, "a habit with this title is already on the board")
            }
            HabitBoardError::BoardFull { max } => {
                write!(
                    f,
                    "the habit board already holds the maximum of {max} habits"
                )
            }
        }
    }
}

impl Error for HabitBoardError {}

impl HabitBoard {
    pub const MAX_HABITS: usize = 5;

    pub fn request_habit(
        &mut self,
        id: HabitId,
        title: String,
        goal: u32,
    ) -> Result<HabitBoardEvent, HabitBoardError> {
        let title = HabitTitle::new(title).map_err(HabitBoardError::InvalidHabit)?;
        let goal = Goal::new(goal).map_err(HabitBoardError::InvalidHabit)?;

        if self
            .requests
            .iter()
            .any(|entry| entry.title.matches(&title))
        {
            return Err(HabitBoardError::DuplicateHabit);
        }

        if self.requests.len() >= Self::MAX_HABITS {
            return Err(HabitBoardError::BoardFull {
                max: Self::MAX_HABITS,
            });
        }

        self.requests.push(BoardEntry {
            id: id.clone(),
            title: title.clone(),
        });

        Ok(HabitBoardEvent::HabitRequested { id, title, goal })
    }

    pub fn release(&mut self, id: &HabitId) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_each_error_variant_with_the_expected_message() {
        let cases = vec![
            (
                HabitBoardError::DuplicateHabit,
                "a habit with this title is already on the board".to_string(),
            ),
            (
                HabitBoardError::BoardFull { max: 5 },
                "the habit board already holds the maximum of 5 habits".to_string(),
            ),
            (
                HabitBoardError::InvalidHabit(HabitError::TitleLength { min: 1, max: 50 }),
                HabitError::TitleLength { min: 1, max: 50 }.to_string(),
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }
}
