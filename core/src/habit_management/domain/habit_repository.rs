use super::habit::Habit;

pub trait HabitRepository {
    fn save(&self, habit: &Habit);
    fn all(&self) -> Vec<Habit>;
}
