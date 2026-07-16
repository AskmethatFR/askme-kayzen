use crate::habit_management::domain::habit_description::HabitDescription;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::initial_duration::InitialDuration;

#[derive(Debug, PartialEq)]
pub enum HabitBoardEvent {
    HabitRequested {
        id: HabitId,
        description: HabitDescription,
        initial_duration: InitialDuration,
    },
}
