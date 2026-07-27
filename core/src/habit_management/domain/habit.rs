use crate::habit_management::domain::completion_history::CompletionHistory;
use crate::habit_management::domain::goal::Goal;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;
use crate::habit_management::domain::step_history::StepHistory;
use crate::shared::local_date::LocalDate;
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct Habit {
    id: HabitId,
    title: HabitTitle,
    steps: StepHistory,
    completion_history: CompletionHistory,
}

#[derive(Debug, PartialEq)]
pub enum HabitError {
    GoalTooSmall { min: u32 },
    TitleLength { min: usize, max: usize },
    IdLength { min: usize, max: usize },
}

impl fmt::Display for HabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HabitError::GoalTooSmall { min } => {
                write!(f, "a goal must be at least {min} minute(s) per day")
            }
            HabitError::TitleLength { min, max } => {
                write!(f, "a title size must be between {min} and {max} characters")
            }
            HabitError::IdLength { min, max } => {
                write!(f, "an id size must be between {min} and {max} characters")
            }
        }
    }
}

impl Error for HabitError {}

impl Habit {
    pub fn new(id: HabitId, title: HabitTitle, goal: Goal, created_on: LocalDate) -> Habit {
        Habit {
            id,
            title,
            steps: StepHistory::seeded(created_on, goal),
            completion_history: CompletionHistory::new(),
        }
    }

    pub fn id(&self) -> &HabitId {
        &self.id
    }
    pub fn title(&self) -> &HabitTitle {
        &self.title
    }
    pub fn current_goal(&self) -> u32 {
        self.steps.current().value()
    }
    pub fn step_history(&self) -> &StepHistory {
        &self.steps
    }

    pub fn toggle_done(&mut self, today: LocalDate) {
        self.completion_history.toggle(today);
    }

    pub fn grow(&mut self, today: LocalDate) {
        self.steps.record(today, self.steps.current().grown());
    }

    pub fn is_done_on(&self, day: LocalDate) -> bool {
        self.completion_history.contains(day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::local_date::LocalDate;

    const CREATED_ON: i64 = 20_000;

    fn a_habit() -> Habit {
        Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(2).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    // Test List — Habit::new seeding a StepHistory at creation (@feature:adjust-goal):
    // - current_goal() reads the goal the habit was created with.
    // - step_history() starts with exactly one step, dated at creation.
    #[test]
    fn current_goal_reads_the_goal_seeded_at_creation() {
        let habit = a_habit();

        assert_eq!(habit.current_goal(), 2);
    }

    #[test]
    fn step_history_is_seeded_with_one_step_dated_at_creation() {
        let habit = a_habit();

        let steps: Vec<(LocalDate, u32)> = habit
            .step_history()
            .changes()
            .into_iter()
            .map(|step| (step.on(), step.goal().value()))
            .collect();

        assert_eq!(steps, vec![(LocalDate::from_epoch_day(CREATED_ON), 2)]);
    }

    #[test]
    fn display_formats_each_error_variant_with_the_expected_message() {
        let cases = vec![
            (
                HabitError::GoalTooSmall { min: 1 },
                "a goal must be at least 1 minute(s) per day",
            ),
            (
                HabitError::TitleLength { min: 1, max: 50 },
                "a title size must be between 1 and 50 characters",
            ),
            (
                HabitError::IdLength { min: 1, max: 64 },
                "an id size must be between 1 and 64 characters",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
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
