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
            MarkDoneError::HabitNotFound => write!(f, "no habit with this id exists"),
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
        let id = HabitId::new(habit_id).map_err(|_| MarkDoneError::HabitNotFound)?;
        let mut habit = self
            .repository
            .get(&id)
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
            HabitId::new(id).unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(2).unwrap(),
            LocalDate::from_epoch_day(TODAY),
        )
    }

    fn mark_done_over(repository: Rc<InMemoryHabitRepository>) -> MarkDone {
        MarkDone::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(TODAY))) as Rc<dyn Clock>,
        )
    }

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            MarkDoneError::HabitNotFound.to_string(),
            "no habit with this id exists"
        );
    }

    // @scenario: mark-done/S1
    #[test]
    fn marking_a_habit_records_todays_completion() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let mark_done = mark_done_over(Rc::clone(&repository));

        let result = mark_done.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
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

        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert!(!habit.is_done_on(LocalDate::from_epoch_day(TODAY)));
    }

    // @scenario: anchor-habit/S3
    #[test]
    fn marking_an_anchored_habit_still_records_todays_completion() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1");
        habit.anchor();
        repository.save(&habit);
        let mark_done = mark_done_over(Rc::clone(&repository));

        let result = mark_done.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert!(
            habit.is_done_on(LocalDate::from_epoch_day(TODAY)),
            "anchoring ends the seat, not the habit — MarkDone must not refuse it"
        );
    }

    // @scenario: mark-done/S3
    #[test]
    fn marking_an_unknown_habit_is_rejected() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mark_done = mark_done_over(repository);

        let result = mark_done.execute("missing");

        assert_eq!(result, Err(MarkDoneError::HabitNotFound));
    }

    // No Gherkin scenario names this path yet either (invalid-id refusal,
    // T1 conformance with adr-0001) — flagged under "Open questions".
    #[test]
    fn marking_an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let mark_done = mark_done_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        let result = mark_done.execute(&too_long);

        assert_eq!(result, Err(MarkDoneError::HabitNotFound));
    }
}
