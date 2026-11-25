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
