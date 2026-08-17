use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;

#[derive(Debug, PartialEq)]
pub enum ResumeHabitError {
    HabitNotFound,
    NotPaused,
}

impl fmt::Display for ResumeHabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResumeHabitError::HabitNotFound => write!(f, "no habit with this id exists"),
            ResumeHabitError::NotPaused => write!(f, "only a paused habit can be resumed"),
        }
    }
}

impl Error for ResumeHabitError {}

/// Command use case: resumes a paused habit. No `Clock` (adr-0007 AD-3):
/// nothing about this transition is dated. Deliberately not shared with
/// `PauseHabit` (adr-0011 lines 82-83 name this slice explicitly).
#[derive(Clone)]
pub struct ResumeHabit {
    repository: Rc<dyn HabitRepository>,
}

impl ResumeHabit {
    pub fn new(repository: Rc<dyn HabitRepository>) -> ResumeHabit {
        ResumeHabit { repository }
    }

    pub fn execute(&self, habit_id: &str) -> Result<(), ResumeHabitError> {
        let id = HabitId::new(habit_id).map_err(|_| ResumeHabitError::HabitNotFound)?;
        let mut habit = self
            .repository
            .get(&id)
            .ok_or(ResumeHabitError::HabitNotFound)?;
        habit
            .resume()
            .map_err(|_| ResumeHabitError::NotPaused)?;
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

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            ResumeHabitError::HabitNotFound.to_string(),
            "no habit with this id exists"
        );
        assert_eq!(
            ResumeHabitError::NotPaused.to_string(),
            "only a paused habit can be resumed"
        );
    }

    fn a_habit(id: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn resume_habit_over(repository: Rc<InMemoryHabitRepository>) -> ResumeHabit {
        ResumeHabit::new(repository as Rc<dyn HabitRepository>)
    }

    // @scenario: pause-resume/S2
    #[test]
    fn resuming_a_paused_habit_makes_it_active_and_its_history_untouched() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1");
        habit.toggle_done(LocalDate::from_epoch_day(CREATED_ON));
        habit.pause().expect("a fresh habit is active");
        repository.save(&habit);
        let resume_habit = resume_habit_over(Rc::clone(&repository));

        let result = resume_habit.execute("h-1");

        assert_eq!(result, Ok(()));
        let resumed = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(resumed.state(), LifecycleState::Active);
        assert!(
            resumed.is_done_on(LocalDate::from_epoch_day(CREATED_ON)),
            "resuming must leave the completion history untouched"
        );
    }

    #[test]
    fn resuming_an_unknown_habit_is_rejected() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let resume_habit = resume_habit_over(repository);

        let result = resume_habit.execute("missing");

        assert_eq!(result, Err(ResumeHabitError::HabitNotFound));
    }

    #[test]
    fn resuming_an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let resume_habit = resume_habit_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        let result = resume_habit.execute(&too_long);

        assert_eq!(result, Err(ResumeHabitError::HabitNotFound));
    }

    #[test]
    fn resuming_an_active_habit_is_refused() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let resume_habit = resume_habit_over(Rc::clone(&repository));

        let result = resume_habit.execute("h-1");

        assert_eq!(result, Err(ResumeHabitError::NotPaused));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.state(), LifecycleState::Active);
    }

    // the security cell: pause -> anchor no longer holds a route back to
    // Active through resume().
    #[test]
    fn resuming_an_anchored_habit_is_refused() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1");
        habit.anchor();
        repository.save(&habit);
        let resume_habit = resume_habit_over(Rc::clone(&repository));

        let result = resume_habit.execute("h-1");

        assert_eq!(result, Err(ResumeHabitError::NotPaused));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.state(), LifecycleState::Anchored);
    }
}
