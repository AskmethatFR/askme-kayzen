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

        habit.record_attempt(self.clock.today());
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
        assert_eq!(habit.attempts_on(LocalDate::from_epoch_day(TODAY)), 1);
    }

    // @scenario: mark-done/S5
    #[test]
    fn marking_the_same_habit_twice_counts_two_attempts_today() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let mark_done = mark_done_over(Rc::clone(&repository));

        mark_done.execute("h-1").unwrap();
        mark_done.execute("h-1").unwrap();

        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert!(habit.is_done_on(LocalDate::from_epoch_day(TODAY)));
        assert_eq!(habit.attempts_on(LocalDate::from_epoch_day(TODAY)), 2);
    }

    // @scenario: mark-done/S6
    #[test]
    fn attempts_past_the_goal_are_counted_never_refused() {
        for (before, after) in [(2, 3), (4, 5)] {
            let repository = Rc::new(InMemoryHabitRepository::new());
            repository.save(&a_habit("h-1"));
            let mark_done = mark_done_over(Rc::clone(&repository));
            for _ in 0..before {
                mark_done.execute("h-1").unwrap();
            }

            let result = mark_done.execute("h-1");

            assert_eq!(result, Ok(()), "row {before} -> {after}");
            let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
            assert_eq!(
                habit.attempts_on(LocalDate::from_epoch_day(TODAY)),
                after,
                "row {before} -> {after}"
            );
        }
    }

    // @scenario: mark-done/S7
    #[test]
    fn a_days_count_belongs_to_one_habit_and_one_local_date() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        repository.save(&a_habit("h-2"));
        let yesterday = LocalDate::from_epoch_day(TODAY - 1);
        MarkDone::new(
            Rc::clone(&repository) as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(yesterday)) as Rc<dyn Clock>,
        )
        .execute("h-1")
        .unwrap();

        mark_done_over(Rc::clone(&repository))
            .execute("h-1")
            .unwrap();

        let first = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        let second = repository.get(&HabitId::new("h-2").unwrap()).unwrap();
        assert_eq!(
            (
                first.attempts_on(LocalDate::from_epoch_day(TODAY)),
                first.attempts_on(yesterday),
                second.attempts_on(LocalDate::from_epoch_day(TODAY)),
            ),
            (1, 1, 0)
        );
    }

    // @scenario: anchor-habit/S3
    #[test]
    fn marking_an_anchored_habit_still_records_todays_completion() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1");
        habit.anchor().expect("a fresh habit is active");
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
        repository.save(&a_habit("h-1"));
        let mark_done = mark_done_over(Rc::clone(&repository));

        let result = mark_done.execute("missing");

        assert_eq!(result, Err(MarkDoneError::HabitNotFound));
        assert_eq!(repository.all(), vec![a_habit("h-1")]);
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
