use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;
use crate::habit_management::domain::initial_duration::InitialDuration;

#[derive(Debug, PartialEq)]
pub enum HabitBoardEvent {
    HabitRequested {
        id: HabitId,
        title: HabitTitle,
        initial_duration: InitialDuration,
    },
}
