use std::cell::RefCell;

use crate::habit_management::domain::habit_board::HabitBoard;
use crate::habit_management::domain::habit_board_repository::HabitBoardRepository;

#[derive(Default)]
pub struct InMemoryHabitBoardRepository {
    board: RefCell<HabitBoard>,
}

impl InMemoryHabitBoardRepository {
    pub fn new() -> InMemoryHabitBoardRepository {
        InMemoryHabitBoardRepository::default()
    }
}

impl HabitBoardRepository for InMemoryHabitBoardRepository {
    fn load(&self) -> HabitBoard {
        self.board.borrow().clone()
    }

    fn save(&self, board: &HabitBoard) {
        *self.board.borrow_mut() = board.clone();
    }
}
