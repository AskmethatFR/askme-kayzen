use crate::habit_management::domain::habit::HabitError;
use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;
use crate::habit_management::domain::initial_duration::InitialDuration;
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
                write!(f, "the habit board already holds the maximum of {max} habits")
            }
        }
    }
}

impl Error for HabitBoardError {}

impl HabitBoard {
    pub const MAX_HABITS: usize = 5;

    pub fn new() -> HabitBoard {
        HabitBoard::default()
    }

    pub fn request_habit(
        &mut self,
        id: HabitId,
        title: String,
        initial_duration: u32,
    ) -> Result<HabitBoardEvent, HabitBoardError> {
        let title = HabitTitle::new(title).map_err(HabitBoardError::InvalidHabit)?;
        let initial_duration =
            InitialDuration::new(initial_duration).map_err(HabitBoardError::InvalidHabit)?;

        if self.requests.iter().any(|entry| entry.title.matches(&title)) {
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

        Ok(HabitBoardEvent::HabitRequested {
            id,
            title,
            initial_duration,
        })
    }
}
