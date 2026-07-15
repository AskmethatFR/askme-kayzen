use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
use crate::habit_management::domain::habit_id::HabitId;

#[derive(Debug, PartialEq)]
pub enum HabitBoardError {}

#[derive(Debug, Default, PartialEq)]
pub struct HabitBoard {}

impl HabitBoard {
    pub fn new() -> HabitBoard {
        HabitBoard::default()
    }

    pub fn request_habit(
        &mut self,
        id: HabitId,
        description: String,
        initial_duration: u32,
    ) -> Result<HabitBoardEvent, HabitBoardError> {
        Ok(HabitBoardEvent::HabitRequested {
            id,
            description,
            initial_duration,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::habit_id::HabitId;

    #[test]
    fn requesting_a_habit_emits_a_creation_request() {
        let mut board = HabitBoard::new();

        let result = board.request_habit(HabitId::from("id-1"), String::from("Read one page"), 2);

        assert_eq!(
            result,
            Ok(HabitBoardEvent::HabitRequested {
                id: HabitId::from("id-1"),
                description: String::from("Read one page"),
                initial_duration: 2,
            })
        );
    }
}
