use crate::habit_management::domain::completion_history::CompletionHistory;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;
use crate::habit_management::domain::initial_duration::InitialDuration;
use crate::shared::local_date::LocalDate;
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct Habit {
    id: HabitId,
    title: HabitTitle,
    initial_duration: InitialDuration,
    completion_history: CompletionHistory,
}

#[derive(Debug, PartialEq)]
pub enum HabitError {
    DurationTooLong { max: u32 },
    TitleLength { min: usize, max: usize },
}

impl fmt::Display for HabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HabitError::DurationTooLong { max } => {
                write!(f, "an easy habit must last no more than {max} minutes")
            }
            HabitError::TitleLength { min, max } => {
                write!(f, "a title size must be between {min} and {max} characters")
            }
        }
    }
}

impl Error for HabitError {}

impl Habit {
    pub fn new(id: HabitId, title: HabitTitle, initial_duration: InitialDuration) -> Habit {
        Habit {
            id,
            title,
            initial_duration,
            completion_history: CompletionHistory::new(),
        }
    }

    pub fn id(&self) -> &HabitId {
        &self.id
    }
    pub fn title(&self) -> &HabitTitle {
        &self.title
    }
    pub fn current_dose(&self) -> u32 {
        self.initial_duration.value()
    }

    pub fn toggle_done(&mut self, today: LocalDate) {
        self.completion_history.toggle(today);
    }

    pub fn is_done_on(&self, day: LocalDate) -> bool {
        self.completion_history.contains(day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::local_date::LocalDate;

    fn a_habit() -> Habit {
        Habit::new(
            HabitId::from("h-1"),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            InitialDuration::new(2).unwrap(),
        )
    }

    #[test]
    fn a_new_habit_is_not_done_on_any_day() {
        let habit = a_habit();

        assert!(!habit.is_done_on(LocalDate::from_epoch_day(20_000)));
    }

    #[test]
    fn toggling_marks_the_habit_done_that_day() {
        let mut habit = a_habit();
        let today = LocalDate::from_epoch_day(20_000);

        habit.toggle_done(today);

        assert!(habit.is_done_on(today));
    }

    #[test]
    fn toggling_again_the_same_day_clears_it() {
        let mut habit = a_habit();
        let today = LocalDate::from_epoch_day(20_000);

        habit.toggle_done(today);
        habit.toggle_done(today);

        assert!(!habit.is_done_on(today));
    }

    #[test]
    fn a_completion_is_scoped_to_its_own_day() {
        let mut habit = a_habit();
        let done_day = LocalDate::from_epoch_day(20_000);
        let other_day = LocalDate::from_epoch_day(20_001);

        habit.toggle_done(done_day);

        assert!(habit.is_done_on(done_day));
        assert!(!habit.is_done_on(other_day));
    }
}
