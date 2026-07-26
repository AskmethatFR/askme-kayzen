use crate::habit_management::domain::goal::Goal;
use crate::shared::local_date::LocalDate;

/// One dated change to a habit's daily goal. The date is what makes a later
/// "minutes gained since day X" query reconstructible (adr-0007) — even
/// though nothing reads it back within this slice yet.
#[derive(Debug, Clone, PartialEq)]
pub struct StepChange {
    on: LocalDate,
    goal: Goal,
}

impl StepChange {
    fn new(on: LocalDate, goal: Goal) -> StepChange {
        StepChange { on, goal }
    }

    pub fn on(&self) -> LocalDate {
        self.on
    }

    pub fn goal(&self) -> &Goal {
        &self.goal
    }
}

/// The dated history of a habit's goal, one step per gesture (`grow`/`lighten`).
/// Non-empty by construction: `seeded` is the only constructor and always
/// plants the first step, so reading the current step never needs `Option`
/// or a panic — `current()` falls back to the always-present first step.
#[derive(Debug, Clone, PartialEq)]
pub struct StepHistory {
    first: StepChange,
    later: Vec<StepChange>,
}

impl StepHistory {
    pub fn seeded(on: LocalDate, goal: Goal) -> StepHistory {
        StepHistory {
            first: StepChange::new(on, goal),
            later: Vec::new(),
        }
    }

    pub fn record(&mut self, on: LocalDate, goal: Goal) {
        self.later.push(StepChange::new(on, goal));
    }

    pub fn current(&self) -> &Goal {
        match self.later.last() {
            Some(step) => step.goal(),
            None => self.first.goal(),
        }
    }

    pub fn changes(&self) -> Vec<&StepChange> {
        std::iter::once(&self.first)
            .chain(self.later.iter())
            .collect()
    }
}
