use crate::habit_management::domain::habit::HabitError;
use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;
use crate::habit_management::domain::initial_duration::InitialDuration;

#[derive(Debug, Default, PartialEq)]
pub struct HabitBoard {}

impl HabitBoard {
    pub fn new() -> HabitBoard {
        HabitBoard::default()
    }

    pub fn request_habit(
        &self,
        id: HabitId,
        title: String,
        initial_duration: u32,
    ) -> Result<HabitBoardEvent, HabitError> {
        let title = HabitTitle::new(title)?;
        let initial_duration = InitialDuration::new(initial_duration)?;

        Ok(HabitBoardEvent::HabitRequested {
            id,
            title,
            initial_duration,
        })
    }
}
