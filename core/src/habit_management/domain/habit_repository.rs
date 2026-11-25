use super::habit::Habit;
use super::habit_id::HabitId;

pub trait HabitRepository {
    fn save(&self, habit: &Habit);
    fn all(&self) -> Vec<Habit>;
    fn get(&self, id: &HabitId) -> Option<Habit>;
}
