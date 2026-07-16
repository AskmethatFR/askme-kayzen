use super::habit_board::HabitBoard;

pub trait HabitBoardRepository {
    fn load(&self) -> HabitBoard;
    fn save(&self, board: &HabitBoard);
}
