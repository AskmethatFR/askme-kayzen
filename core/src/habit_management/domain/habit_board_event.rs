use crate::habit_management::domain::goal::Goal;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;

#[derive(Debug, PartialEq)]
pub enum HabitBoardEvent {
    HabitRequested {
        id: HabitId,
        title: HabitTitle,
        goal: Goal,
    },
}
