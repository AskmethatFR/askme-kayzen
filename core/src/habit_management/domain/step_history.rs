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

/// The dated history of a habit's goal. Task 1 seeds exactly one step at
/// creation (adr-0007/adr-0008 AD-1) — `seeded` is the only constructor, so
/// non-emptiness stays structural and reading the current step never needs
/// `Option` or a panic. Further steps are appended one at a time through
/// `record`, called by the use case that grows or lightens the goal
/// (adjust-goal slice 3); the history never removes, pops, or merges steps.
#[derive(Debug, Clone, PartialEq)]
pub struct StepHistory {
    first: StepChange,
    rest: Vec<StepChange>,
}

impl StepHistory {
    pub fn seeded(on: LocalDate, goal: Goal) -> StepHistory {
        StepHistory {
            first: StepChange::new(on, goal),
            rest: Vec::new(),
        }
    }

    pub fn current(&self) -> &Goal {
        self.rest
            .last()
            .map(StepChange::goal)
            .unwrap_or_else(|| self.first.goal())
    }

    pub fn changes(&self) -> Vec<&StepChange> {
        let mut changes = vec![&self.first];
        changes.extend(self.rest.iter());
        changes
    }

    pub fn record(&mut self, on: LocalDate, goal: Goal) {
        self.rest.push(StepChange::new(on, goal));
    }
}
