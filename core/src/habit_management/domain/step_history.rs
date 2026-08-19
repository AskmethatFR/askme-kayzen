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

    pub fn started_on(&self) -> LocalDate {
        self.first.on()
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
        if &goal == self.current() {
            return;
        }
        self.rest.push(StepChange::new(on, goal));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_history() -> StepHistory {
        StepHistory::seeded(LocalDate::from_epoch_day(20_000), Goal::new(5).unwrap())
    }

    // Called by two use cases (GrowGoal, LightenGoal — via Habit::grow/lighten)
    // and read by GetHabitDetail: a published contract, so this pins the
    // invariant directly rather than through a single calling use case
    // (test-ddd-tactical Entry Gate). This test drove the guard into record()
    // itself (N1) — Habit no longer duplicates it as already_at_the_ceiling/
    // already_at_the_floor on the caller side.
    #[test]
    fn recording_a_goal_equal_to_the_current_one_leaves_the_history_unchanged() {
        let mut history = a_history();

        history.record(LocalDate::from_epoch_day(20_001), Goal::new(5).unwrap());

        assert_eq!(history.changes().len(), 1);
        assert_eq!(history.current(), &Goal::new(5).unwrap());
    }

    #[test]
    fn recording_a_different_goal_appends_a_new_step() {
        let mut history = a_history();

        history.record(LocalDate::from_epoch_day(20_001), Goal::new(6).unwrap());

        assert_eq!(history.changes().len(), 2);
        assert_eq!(history.current(), &Goal::new(6).unwrap());
    }
}
