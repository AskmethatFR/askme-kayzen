use crate::habit_management::domain::goal::Goal;
use crate::shared::local_date::LocalDate;

/// One dated change to a habit's daily goal. The date is what makes
/// `StepHistory::goal_on` able to reconstruct the goal in force on any past
/// day (adr-0007).
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
/// A step recording the goal already in force is not a step — `record`
/// ignores it, so no two consecutive steps ever carry the same goal.
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

    /// The goal that was in force on `day`: the last step dated on or before
    /// it, falling back to the seeded one. Indexing the seeded step cannot
    /// panic — `seeded` is the type's only constructor, so `first` always
    /// exists (see the type doc comment).
    pub fn goal_on(&self, day: LocalDate) -> &Goal {
        self.rest
            .iter()
            .rev()
            .find(|step| step.on() <= day)
            .map(StepChange::goal)
            .unwrap_or_else(|| self.first.goal())
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
    // (test-ddd-tactical Entry Gate).
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

    // Test List — StepHistory::goal_on (published contract: called directly by
    // GetHabitDetail via `step_history()`, and by Habit::minutes_practised —
    // test-ddd-tactical Entry Gate, same shape as `record` above):
    // - a day older than the seeded step falls back to the seeded goal.
    // - a day exactly on the seeded step's date returns the seeded goal.
    // - a day after a later recorded step returns that step's goal.
    // - a day between two recorded steps returns the earlier one still in force.
    // - several applicable steps -> returns the most recent one in force.

    #[test]
    fn a_day_older_than_the_seeded_step_falls_back_to_the_seeded_goal() {
        let history = a_history();

        let goal = history.goal_on(LocalDate::from_epoch_day(19_999));

        assert_eq!(goal, &Goal::new(5).unwrap());
    }

    #[test]
    fn a_day_exactly_on_the_seeded_steps_date_returns_the_seeded_goal() {
        let history = a_history();

        let goal = history.goal_on(LocalDate::from_epoch_day(20_000));

        assert_eq!(goal, &Goal::new(5).unwrap());
    }

    #[test]
    fn a_day_after_a_later_recorded_step_returns_that_steps_goal() {
        let mut history = a_history();
        history.record(LocalDate::from_epoch_day(20_003), Goal::new(6).unwrap());

        let goal = history.goal_on(LocalDate::from_epoch_day(20_005));

        assert_eq!(goal, &Goal::new(6).unwrap());
    }

    #[test]
    fn a_day_between_two_recorded_steps_returns_the_earlier_one_still_in_force() {
        let mut history = a_history();
        history.record(LocalDate::from_epoch_day(20_003), Goal::new(6).unwrap());
        history.record(LocalDate::from_epoch_day(20_010), Goal::new(7).unwrap());

        let goal = history.goal_on(LocalDate::from_epoch_day(20_005));

        assert_eq!(
            goal,
            &Goal::new(6).unwrap(),
            "day 20_005 sits after the 20_003 step but before the 20_010 one"
        );
    }

    #[test]
    fn several_applicable_steps_return_the_most_recent_one_in_force() {
        let mut history = a_history();
        history.record(LocalDate::from_epoch_day(20_003), Goal::new(6).unwrap());
        history.record(LocalDate::from_epoch_day(20_010), Goal::new(7).unwrap());

        let goal = history.goal_on(LocalDate::from_epoch_day(20_020));

        assert_eq!(goal, &Goal::new(7).unwrap());
    }
}
