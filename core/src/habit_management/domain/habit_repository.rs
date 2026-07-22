use super::habit::Habit;
use super::habit_id::HabitId;

pub trait HabitRepository {
    /// Upsert by id: saving a habit whose id already exists overwrites it,
    /// so a mutated aggregate (e.g. after `toggle_done`) replaces its stored copy.
    fn save(&self, habit: &Habit);
    fn all(&self) -> Vec<Habit>;
    fn get(&self, id: &HabitId) -> Option<Habit>;
}
