use super::habit::Habit;
use super::habit_id::HabitId;

pub trait HabitRepository {
    // @law: upsert by id; saving an existing id replaces the stored copy, so a mutated aggregate never duplicates.
    fn save(&self, habit: &Habit);
    fn all(&self) -> Vec<Habit>;
    fn get(&self, id: &HabitId) -> Option<Habit>;
}
