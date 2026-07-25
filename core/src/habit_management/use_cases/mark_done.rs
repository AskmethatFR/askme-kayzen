use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::shared::clock::Clock;

#[derive(Debug, PartialEq)]
pub enum MarkDoneError {
    HabitNotFound,
}

impl fmt::Display for MarkDoneError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MarkDoneError::HabitNotFound => write!(f, "no habit with this id is on the board"),
        }
    }
}

impl Error for MarkDoneError {}

/// Command use case: toggles today's completion for one habit. Loads the right
/// habit, applies the domain method with the clock's "today", saves the mutated
/// aggregate (upsert). Lifecycle change stays internal — nothing is published.
#[derive(Clone)]
pub struct MarkDone {
    repository: Rc<dyn HabitRepository>,
    clock: Rc<dyn Clock>,
}

impl MarkDone {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> MarkDone {
        MarkDone { repository, clock }
    }

    pub fn execute(&self, habit_id: &str) -> Result<(), MarkDoneError> {
        let mut habit = self
            .repository
            .get(&HabitId::from(habit_id))
            .ok_or(MarkDoneError::HabitNotFound)?;

        habit.toggle_done(self.clock.today());
        self.repository.save(&habit);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::shared::clock::FixedClock;
    use crate::shared::local_date::LocalDate;

    const TODAY: i64 = 20_000;

    fn a_habit(id: &str) -> Habit {
        Habit::new(
            HabitId::from(id),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(2).unwrap(),
        )
    }

    fn mark_done_over(repository: Rc<InMemoryHabitRepository>) -> MarkDone {
        MarkDone::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(TODAY))) as Rc<dyn Clock>,
        )
    }

    // @scenario: mark-done/S1
    #[test]
    fn marking_a_habit_records_todays_completion() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let mark_done = mark_done_over(Rc::clone(&repository));

        let result = mark_done.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::from("h-1")).unwrap();
        assert!(habit.is_done_on(LocalDate::from_epoch_day(TODAY)));
    }

    // @scenario: mark-done/S2
    #[test]
    fn marking_the_same_habit_twice_clears_todays_completion() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let mark_done = mark_done_over(Rc::clone(&repository));

        mark_done.execute("h-1").unwrap();
        mark_done.execute("h-1").unwrap();

        let habit = repository.get(&HabitId::from("h-1")).unwrap();
        assert!(!habit.is_done_on(LocalDate::from_epoch_day(TODAY)));
    }

    // @scenario: mark-done/S3
    #[test]
    fn marking_an_unknown_habit_is_rejected() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mark_done = mark_done_over(repository);

        let result = mark_done.execute("missing");

        assert_eq!(result, Err(MarkDoneError::HabitNotFound));
    }
}
