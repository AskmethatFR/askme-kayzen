use std::cell::RefCell;

use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_repository::HabitRepository;

#[derive(Default)]
pub struct InMemoryHabitRepository {
    habits: RefCell<Vec<Habit>>,
}

impl InMemoryHabitRepository {
    pub fn new() -> InMemoryHabitRepository {
        InMemoryHabitRepository::default()
    }
}

impl HabitRepository for InMemoryHabitRepository {
    fn save(&self, habit: &Habit) {
        self.habits.borrow_mut().push(habit.clone());
    }

    fn all(&self) -> Vec<Habit> {
        self.habits.borrow().clone()
    }
}
