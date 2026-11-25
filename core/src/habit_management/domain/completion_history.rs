use std::collections::BTreeMap;

use crate::shared::local_date::LocalDate;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CompletionHistory(BTreeMap<LocalDate, u32>);

impl CompletionHistory {
    pub fn new() -> CompletionHistory {
        CompletionHistory::default()
    }

    pub fn record(&mut self, day: LocalDate) {
        let attempts = self.0.entry(day).or_insert(0);
        *attempts = attempts.saturating_add(1);
    }

    pub fn withdraw(&mut self, day: LocalDate) {
        match self.0.get_mut(&day) {
            Some(attempts) if *attempts > 1 => *attempts -= 1,
            Some(_) => {
                self.0.remove(&day);
            }
            None => {}
        }
    }

    #[cfg(test)]
    pub(crate) fn with_attempts(day: LocalDate, attempts: u32) -> CompletionHistory {
        CompletionHistory(BTreeMap::from([(day, attempts)]))
    }

    pub fn contains(&self, day: LocalDate) -> bool {
        self.0.contains_key(&day)
    }

    pub fn attempts_on(&self, day: LocalDate) -> u32 {
        self.0.get(&day).copied().unwrap_or(0)
    }

    pub(crate) fn dates(&self) -> impl Iterator<Item = LocalDate> + '_ {
        self.0
            .iter()
            .flat_map(|(day, attempts)| std::iter::repeat_n(*day, *attempts as usize))
    }

    // @law: snapshot v1 stores a day once per attempt; a payload written before counts holds unique days, one attempt each.
    pub(crate) fn rehydrate(dates: impl IntoIterator<Item = LocalDate>) -> CompletionHistory {
        let mut history = CompletionHistory::new();
        for day in dates {
            history.record(day);
        }
        history
    }

    pub fn attempts_between(&self, from: LocalDate, to: LocalDate) -> u32 {
        self.0
            .range(from..=to)
            .fold(0u32, |total, (_, attempts)| total.saturating_add(*attempts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 20_000;

    #[test]
    fn withdrawing_from_two_attempts_leaves_one_and_the_day_present() {
        let day = LocalDate::from_epoch_day(DAY);
        let mut history = CompletionHistory::with_attempts(day, 2);

        history.withdraw(day);

        assert_eq!(history.attempts_on(day), 1);
        assert!(history.contains(day));
    }

    #[test]
    fn withdrawing_the_only_attempt_removes_the_day_entry() {
        let day = LocalDate::from_epoch_day(DAY);
        let mut history = CompletionHistory::with_attempts(day, 1);

        history.withdraw(day);

        assert!(!history.contains(day));
        assert_eq!(history, CompletionHistory::new());
    }

    #[test]
    fn withdrawing_from_a_day_without_attempts_changes_nothing() {
        let day = LocalDate::from_epoch_day(DAY);
        let other = LocalDate::from_epoch_day(DAY - 1);
        let mut history = CompletionHistory::with_attempts(other, 3);

        history.withdraw(day);

        assert_eq!(history, CompletionHistory::with_attempts(other, 3));
    }
}
