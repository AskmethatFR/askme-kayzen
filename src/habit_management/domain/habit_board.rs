use crate::habit_management::domain::habit::HabitError;
use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
use crate::habit_management::domain::habit_description::HabitDescription;
use crate::habit_management::domain::habit_id::HabitId;
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
        description: String,
        initial_duration: u32,
    ) -> Result<HabitBoardEvent, HabitError> {
        let description = HabitDescription::new(description)?;
        let initial_duration = InitialDuration::new(initial_duration)?;

        Ok(HabitBoardEvent::HabitRequested {
            id,
            description,
            initial_duration,
        })
    }
}
