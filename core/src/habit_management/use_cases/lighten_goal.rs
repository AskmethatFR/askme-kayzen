use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::shared::clock::Clock;

#[derive(Debug, PartialEq)]
pub enum LightenGoalError {
    HabitNotFound,
}

impl fmt::Display for LightenGoalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LightenGoalError::HabitNotFound => write!(f, "no habit with this id is on the board"),
        }
    }
}

impl Error for LightenGoalError {}

/// Command use case: lowers a habit's goal by one minute ("alléger", adr-0008).
/// Loads the right habit, applies the domain method with the clock's "today",
/// saves the mutated aggregate.
#[derive(Clone)]
pub struct LightenGoal {
    repository: Rc<dyn HabitRepository>,
    clock: Rc<dyn Clock>,
}

impl LightenGoal {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> LightenGoal {
        LightenGoal { repository, clock }
    }

    pub fn execute(&self, habit_id: &str) -> Result<(), LightenGoalError> {
        let id = HabitId::new(habit_id).map_err(|_| LightenGoalError::HabitNotFound)?;
        let mut habit = self
            .repository
            .get(&id)
            .ok_or(LightenGoalError::HabitNotFound)?;

        habit.lighten(self.clock.today());
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

    const CREATED_ON: i64 = 19_990;
    const TODAY: i64 = 20_000;

    fn a_habit(id: &str, goal: u32) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(goal).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn dated_steps(habit: &Habit) -> Vec<(LocalDate, u32)> {
        habit
            .step_history()
            .changes()
            .into_iter()
            .map(|step| (step.on(), step.goal().value()))
            .collect()
    }

    fn lighten_goal_over(repository: Rc<InMemoryHabitRepository>) -> LightenGoal {
        LightenGoal::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(TODAY))) as Rc<dyn Clock>,
        )
    }

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            LightenGoalError::HabitNotFound.to_string(),
            "no habit with this id is on the board"
        );
    }

    // @scenario: adjust-goal/S2
    #[test]
    fn lightening_a_habit_lowers_its_goal_and_records_todays_step() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5));
        let lighten_goal = lighten_goal_over(Rc::clone(&repository));

        let result = lighten_goal.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.current_goal(), 4);
        assert_eq!(
            dated_steps(&habit),
            vec![
                (LocalDate::from_epoch_day(CREATED_ON), 5),
                (LocalDate::from_epoch_day(TODAY), 4),
            ]
        );
    }

    // @scenario: adjust-goal/S3
    #[test]
    fn lightening_a_habit_already_at_the_floor_changes_nothing() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 1));
        let lighten_goal = lighten_goal_over(Rc::clone(&repository));

        let result = lighten_goal.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.current_goal(), 1);
        assert_eq!(
            dated_steps(&habit),
            vec![(LocalDate::from_epoch_day(CREATED_ON), 1)]
        );
    }

    // No Gherkin scenario names this path yet (unknown-habit refusal, mirrors
    // mark-done/S3 and grow-goal's equivalent) — flagged under "Open questions".
    #[test]
    fn lightening_an_unknown_habit_is_rejected() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let lighten_goal = lighten_goal_over(repository);

        let result = lighten_goal.execute("missing");

        assert_eq!(result, Err(LightenGoalError::HabitNotFound));
    }

    // No Gherkin scenario names this path yet either (invalid-id refusal,
    // T1 conformance with adr-0001) — flagged under "Open questions".
    #[test]
    fn lightening_an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5));
        let lighten_goal = lighten_goal_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        let result = lighten_goal.execute(&too_long);

        assert_eq!(result, Err(LightenGoalError::HabitNotFound));
    }
}
