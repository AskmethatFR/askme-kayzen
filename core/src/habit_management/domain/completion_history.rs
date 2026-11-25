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

    pub fn contains(&self, day: LocalDate) -> bool {
        self.0.contains_key(&day)
    }

    pub fn attempts_on(&self, day: LocalDate) -> u32 {
        self.0.get(&day).copied().unwrap_or(0)
    }

    pub(crate) fn dates(&self) -> impl Iterator<Item = LocalDate> + '_ {
        self.0.keys().copied()
    }

    pub(crate) fn rehydrate(dates: impl IntoIterator<Item = LocalDate>) -> CompletionHistory {
        CompletionHistory(dates.into_iter().map(|day| (day, 1)).collect())
    }

    pub fn between(&self, from: LocalDate, to: LocalDate) -> impl Iterator<Item = LocalDate> + '_ {
        self.0.range(from..=to).map(|(day, _)| *day)
    }
}
