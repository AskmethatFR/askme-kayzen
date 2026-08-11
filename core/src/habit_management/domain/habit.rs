use crate::habit_management::domain::completion_history::CompletionHistory;
use crate::habit_management::domain::goal::Goal;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;
use crate::habit_management::domain::lifecycle_state::LifecycleState;
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
    state: LifecycleState,
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
            state: LifecycleState::Active,
        }
    }

    pub fn id(&self) -> &HabitId {
        &self.id
    }
    pub fn title(&self) -> &HabitTitle {
        &self.title
    }
    pub fn state(&self) -> LifecycleState {
        self.state
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

    pub fn pause(&mut self) {
        self.state = LifecycleState::Paused;
    }

    pub fn resume(&mut self) {
        self.state = LifecycleState::Active;
    }

    pub fn anchor(&mut self) {
        todo!()
    }

    pub fn grow(&mut self, today: LocalDate) {
        let grown = self.steps.current().grown();
        let already_at_the_ceiling = grown == *self.steps.current();
        if already_at_the_ceiling {
            return;
        }
        self.steps.record(today, grown);
    }

    pub fn lighten(&mut self, today: LocalDate) {
        let lightened = self.steps.current().lightened();
        let already_at_the_floor = lightened == *self.steps.current();
        if already_at_the_floor {
            return;
        }
        self.steps.record(today, lightened);
    }

    pub fn is_done_on(&self, day: LocalDate) -> bool {
        self.completion_history.contains(day)
    }
}
