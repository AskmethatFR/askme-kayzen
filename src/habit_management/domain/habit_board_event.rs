use crate::habit_management::domain::habit_id::HabitId;

#[derive(Debug, PartialEq)]
pub enum HabitBoardEvent {
    HabitRequested {
        id: HabitId,
        description: String,
        initial_duration: u32,
    },
}
