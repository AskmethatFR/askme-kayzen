use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::shared::clock::Clock;

#[derive(Debug, PartialEq)]
pub enum UndoAttemptError {
    HabitNotFound,
}

impl fmt::Display for UndoAttemptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UndoAttemptError::HabitNotFound => write!(f, "no habit with this id exists"),
        }
    }
}

impl Error for UndoAttemptError {}

#[derive(Clone)]
pub struct UndoAttempt {
    repository: Rc<dyn HabitRepository>,
    clock: Rc<dyn Clock>,
}

impl UndoAttempt {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> UndoAttempt {
        UndoAttempt { repository, clock }
    }

    pub fn execute(&self, habit_id: &str) -> Result<(), UndoAttemptError> {
        let id = HabitId::new(habit_id).map_err(|_| UndoAttemptError::HabitNotFound)?;
        let mut habit = self
            .repository
            .get(&id)
            .ok_or(UndoAttemptError::HabitNotFound)?;

        habit.undo_attempt(self.clock.today());
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

    fn today() -> LocalDate {
        LocalDate::from_epoch_day(TODAY)
    }

    fn a_habit(id: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(2).unwrap(),
            today(),
        )
    }

    fn a_habit_with_attempts_today(id: &str, attempts: u32) -> Habit {
        let mut habit = a_habit(id);
        for _ in 0..attempts {
            habit.record_attempt(today());
        }
        habit
    }

    fn undo_attempt_over(repository: Rc<InMemoryHabitRepository>) -> UndoAttempt {
        UndoAttempt::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(today())) as Rc<dyn Clock>,
        )
    }

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            UndoAttemptError::HabitNotFound.to_string(),
            "no habit with this id exists"
        );
    }

    // @scenario: undo-attempt/S1
    #[test]
    fn undoing_one_of_two_attempts_leaves_one_the_day_practised_and_below_the_goal() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit_with_attempts_today("h-1", 2));
        let undo = undo_attempt_over(Rc::clone(&repository));

        let result = undo.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.attempts_on(today()), 1);
        assert!(habit.is_done_on(today()));
        assert!(!habit.reached_goal_on(today()));
    }

    #[test]
    fn undoing_today_leaves_another_days_count_unchanged() {
        let yesterday = LocalDate::from_epoch_day(TODAY - 1);
        let mut habit = a_habit_with_attempts_today("h-1", 2);
        habit.record_attempt(yesterday);
        habit.record_attempt(yesterday);
        habit.record_attempt(yesterday);
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&habit);
        let undo = undo_attempt_over(Rc::clone(&repository));

        undo.execute("h-1").unwrap();

        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.attempts_on(today()), 1);
        assert_eq!(habit.attempts_on(yesterday), 3);
    }

    // @scenario: undo-attempt/S2
    #[test]
    fn undoing_the_only_attempt_clears_today_and_records_no_day() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit_with_attempts_today("h-1", 1));
        let undo = undo_attempt_over(Rc::clone(&repository));

        let result = undo.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.attempts_on(today()), 0);
        assert!(!habit.is_done_on(today()));
        assert_eq!(habit, a_habit("h-1"));
    }

    // @scenario: undo-attempt/S3
    #[test]
    fn undoing_with_no_attempt_today_changes_nothing_and_says_nothing() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let undo = undo_attempt_over(Rc::clone(&repository));

        let result = undo.execute("h-1");

        assert_eq!(result, Ok(()));
        assert_eq!(repository.all(), vec![a_habit("h-1")]);
    }

    // @scenario: undo-attempt/S5
    #[test]
    fn undoing_an_unknown_habit_is_rejected_and_nothing_changes() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit_with_attempts_today("h-1", 2));
        let undo = undo_attempt_over(Rc::clone(&repository));

        let result = undo.execute("missing");

        assert_eq!(result, Err(UndoAttemptError::HabitNotFound));
        assert_eq!(
            repository.all(),
            vec![a_habit_with_attempts_today("h-1", 2)]
        );
    }

    #[test]
    fn undoing_an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit_with_attempts_today("h-1", 1));
        let undo = undo_attempt_over(Rc::clone(&repository));
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        let result = undo.execute(&too_long);

        assert_eq!(result, Err(UndoAttemptError::HabitNotFound));
        assert_eq!(
            repository.all(),
            vec![a_habit_with_attempts_today("h-1", 1)]
        );
    }
}
