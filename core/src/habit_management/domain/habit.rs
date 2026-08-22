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

#[derive(Debug, PartialEq)]
pub enum TransitionError {
    NotActive,
    NotPaused,
    NotAnchored,
}

impl Habit {
    pub const MAX_IN_DAILY_LIFE: usize = 5;

    pub fn new(id: HabitId, title: HabitTitle, goal: Goal, created_on: LocalDate) -> Habit {
        Habit {
            id,
            title,
            steps: StepHistory::seeded(created_on, goal),
            completion_history: CompletionHistory::new(),
            state: LifecycleState::Active,
        }
    }

    /// Reconstructs a habit from already-parsed value objects and an
    /// already-rebuilt history, for the persistence codec only. `pub(crate)`,
    /// never `pub`: `kayzen-app` must not gain a second door onto `Habit`
    /// construction next to `AddHabit` (adr-0010's single-door rule). Named
    /// `rehydrate` rather than `new` so cargo-mutants — which hard-skips any
    /// method literally called `new` — can still measure this path
    /// (adr-0009).
    pub(crate) fn rehydrate(
        id: HabitId,
        title: HabitTitle,
        steps: StepHistory,
        completion_history: CompletionHistory,
        state: LifecycleState,
    ) -> Habit {
        Habit {
            id,
            title,
            steps,
            completion_history,
            state,
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
    /// `pub(crate)`: the persistence codec's only way to enumerate completion
    /// dates for serialization. `kayzen-app` keeps reading through
    /// `is_done_on`.
    pub(crate) fn completion_history(&self) -> &CompletionHistory {
        &self.completion_history
    }
    pub fn created_on(&self) -> LocalDate {
        self.steps.started_on()
    }

    pub fn toggle_done(&mut self, today: LocalDate) {
        self.completion_history.toggle(today);
    }

    pub fn pause(&mut self) -> Result<(), TransitionError> {
        if self.state != LifecycleState::Active {
            return Err(TransitionError::NotActive);
        }
        self.state = LifecycleState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), TransitionError> {
        if self.state != LifecycleState::Paused {
            return Err(TransitionError::NotPaused);
        }
        self.state = LifecycleState::Active;
        Ok(())
    }

    pub fn anchor(&mut self) -> Result<(), TransitionError> {
        if self.state != LifecycleState::Active {
            return Err(TransitionError::NotActive);
        }
        self.state = LifecycleState::Anchored;
        Ok(())
    }

    pub fn readmit(&mut self) -> Result<(), TransitionError> {
        if self.state != LifecycleState::Anchored {
            return Err(TransitionError::NotAnchored);
        }
        self.state = LifecycleState::Active;
        Ok(())
    }

    pub fn grow(&mut self, today: LocalDate) {
        self.steps.record(today, self.steps.current().grown());
    }

    pub fn lighten(&mut self, today: LocalDate) {
        self.steps.record(today, self.steps.current().lightened());
    }

    pub fn is_done_on(&self, day: LocalDate) -> bool {
        self.completion_history.contains(day)
    }

    /// Minutes practised from creation through `today`, each completed day
    /// weighed by the goal that was in force on it (never today's goal).
    /// Walks the recorded completions, not the calendar, so the cost is
    /// bounded by how much was actually practised rather than the device
    /// clock's distance from `created_on`.
    ///
    /// Clock skew (device date moved back, westward TZ change on the
    /// creation day) can put `today` before `created_on`. Clamping the
    /// range's end to whichever is later keeps it covering at least the
    /// creation day, instead of an inverted range that silently drops it.
    pub fn minutes_practised(&self, today: LocalDate) -> u32 {
        let created_on = self.created_on();
        self.completion_history
            .between(created_on, today.max(created_on))
            .fold(0u32, |minutes, day| {
                minutes.saturating_add(self.steps.goal_on(day).value())
            })
    }
}
