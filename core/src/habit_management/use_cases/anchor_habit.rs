use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;

#[derive(Debug, PartialEq)]
pub enum AnchorHabitError {
    HabitNotFound,
}

impl fmt::Display for AnchorHabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AnchorHabitError::HabitNotFound => write!(f, "no habit with this id is on the board"),
        }
    }
}

impl Error for AnchorHabitError {}

/// Command use case: anchors a habit that has become natural. No `Clock`
/// (adr-0007 AD-3): nothing about this transition is dated.
#[derive(Clone)]
pub struct AnchorHabit {
    repository: Rc<dyn HabitRepository>,
}

impl AnchorHabit {
    pub fn new(repository: Rc<dyn HabitRepository>) -> AnchorHabit {
        AnchorHabit { repository }
    }

    pub fn execute(&self, habit_id: &str) -> Result<(), AnchorHabitError> {
        let id = HabitId::new(habit_id).map_err(|_| AnchorHabitError::HabitNotFound)?;
        let mut habit = self
            .repository
            .get(&id)
            .ok_or(AnchorHabitError::HabitNotFound)?;
        habit.anchor();
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
    use crate::habit_management::domain::lifecycle_state::LifecycleState;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::shared::local_date::LocalDate;

    const CREATED_ON: i64 = 19_990;

    fn a_habit(id: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn anchor_habit_over(repository: Rc<InMemoryHabitRepository>) -> AnchorHabit {
        AnchorHabit::new(repository as Rc<dyn HabitRepository>)
    }

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            AnchorHabitError::HabitNotFound.to_string(),
            "no habit with this id is on the board"
        );
    }

    // @scenario: anchor-habit/S2
    #[test]
    fn anchoring_an_active_habit_marks_it_anchored_in_the_store() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let anchor_habit = anchor_habit_over(Rc::clone(&repository));

        let result = anchor_habit.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.state(), LifecycleState::Anchored);
    }

    #[test]
    fn anchoring_an_unknown_habit_is_rejected() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let anchor_habit = anchor_habit_over(repository);

        let result = anchor_habit.execute("missing");

        assert_eq!(result, Err(AnchorHabitError::HabitNotFound));
    }

    #[test]
    fn anchoring_an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let anchor_habit = anchor_habit_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        let result = anchor_habit.execute(&too_long);

        assert_eq!(result, Err(AnchorHabitError::HabitNotFound));
    }
}
