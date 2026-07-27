use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::shared::clock::Clock;

#[derive(Debug, PartialEq)]
pub enum GrowGoalError {
    HabitNotFound,
}

impl fmt::Display for GrowGoalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GrowGoalError::HabitNotFound => write!(f, "no habit with this id is on the board"),
        }
    }
}

impl Error for GrowGoalError {}

/// Command use case: raises a habit's goal by one minute ("grandir", adr-0008).
/// Loads the right habit, applies the domain method with the clock's "today",
/// saves the mutated aggregate.
#[derive(Clone)]
pub struct GrowGoal {
    repository: Rc<dyn HabitRepository>,
    clock: Rc<dyn Clock>,
}

impl GrowGoal {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> GrowGoal {
        GrowGoal { repository, clock }
    }

    pub fn execute(&self, habit_id: &str) -> Result<(), GrowGoalError> {
        let id = HabitId::new(habit_id).map_err(|_| GrowGoalError::HabitNotFound)?;
        let mut habit = self
            .repository
            .get(&id)
            .ok_or(GrowGoalError::HabitNotFound)?;

        habit.grow(self.clock.today());
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

    fn grow_goal_over(repository: Rc<InMemoryHabitRepository>) -> GrowGoal {
        GrowGoal::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(TODAY))) as Rc<dyn Clock>,
        )
    }

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            GrowGoalError::HabitNotFound.to_string(),
            "no habit with this id is on the board"
        );
    }

    // @scenario: adjust-goal/S1
    #[test]
    fn growing_a_habit_raises_its_goal_and_records_todays_step() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5));
        let grow_goal = grow_goal_over(Rc::clone(&repository));

        let result = grow_goal.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.current_goal(), 6);
        assert_eq!(
            dated_steps(&habit),
            vec![
                (LocalDate::from_epoch_day(CREATED_ON), 5),
                (LocalDate::from_epoch_day(TODAY), 6),
            ]
        );
    }

    // Pins two decisions the S1 test alone cannot discriminate (d3, human
    // arbitration on Dev-B's cross-review): growing twice on the same day
    // APPENDS a distinct step each time — it never overwrites the previous one
    // and never merges same-day steps into one.
    #[test]
    fn growing_twice_on_the_same_day_appends_two_distinct_steps() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5));
        let grow_goal = grow_goal_over(Rc::clone(&repository));

        grow_goal.execute("h-1").unwrap();
        grow_goal.execute("h-1").unwrap();

        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(
            dated_steps(&habit),
            vec![
                (LocalDate::from_epoch_day(CREATED_ON), 5),
                (LocalDate::from_epoch_day(TODAY), 6),
                (LocalDate::from_epoch_day(TODAY), 7),
            ]
        );
    }

    // No Gherkin scenario names this path yet (unknown-habit refusal, mirrors
    // mark-done/S3) — flagged under "Open questions".
    #[test]
    fn growing_an_unknown_habit_is_rejected() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let grow_goal = grow_goal_over(repository);

        let result = grow_goal.execute("missing");

        assert_eq!(result, Err(GrowGoalError::HabitNotFound));
    }

    // No Gherkin scenario names this path yet either (invalid-id refusal,
    // T1 conformance with adr-0001) — flagged under "Open questions".
    #[test]
    fn growing_an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5));
        let grow_goal = grow_goal_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        let result = grow_goal.execute(&too_long);

        assert_eq!(result, Err(GrowGoalError::HabitNotFound));
    }

    // No Gherkin scenario names this path yet (ceiling saturation) — flagged
    // under "Open questions". Unlike `lighten`, `grow` has no early-return
    // guard for a no-op change, so growing at the ceiling still records a
    // step — same goal value, new date — rather than leaving the history
    // untouched the way lightening at the floor does.
    #[test]
    fn growing_a_habit_already_at_the_ceiling_saturates_the_goal_but_still_records_a_step() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", u32::MAX));
        let grow_goal = grow_goal_over(Rc::clone(&repository));

        let result = grow_goal.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.current_goal(), u32::MAX);
        assert_eq!(
            dated_steps(&habit),
            vec![
                (LocalDate::from_epoch_day(CREATED_ON), u32::MAX),
                (LocalDate::from_epoch_day(TODAY), u32::MAX),
            ]
        );
    }
}
