use std::cell::RefCell;

use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_id::HabitId;
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
        let mut habits = self.habits.borrow_mut();
        match habits.iter_mut().find(|stored| stored.id() == habit.id()) {
            Some(stored) => *stored = habit.clone(),
            None => habits.push(habit.clone()),
        }
    }

    fn all(&self) -> Vec<Habit> {
        self.habits.borrow().clone()
    }

    fn get(&self, id: &HabitId) -> Option<Habit> {
        self.habits
            .borrow()
            .iter()
            .find(|habit| habit.id() == id)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::shared::local_date::LocalDate;

    fn a_habit(id: &str) -> Habit {
        Habit::new(
            HabitId::from(id),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(2).unwrap(),
        )
    }

    #[test]
    fn get_returns_none_for_an_unknown_id() {
        let repository = InMemoryHabitRepository::new();

        assert_eq!(repository.get(&HabitId::from("missing")), None);
    }

    #[test]
    fn saving_an_existing_id_overwrites_instead_of_duplicating() {
        let repository = InMemoryHabitRepository::new();
        let mut habit = a_habit("h-1");
        repository.save(&habit);

        habit.toggle_done(LocalDate::from_epoch_day(20_000));
        repository.save(&habit);

        assert_eq!(repository.all().len(), 1);
        assert_eq!(repository.get(&HabitId::from("h-1")), Some(habit));
    }
}
