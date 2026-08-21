use std::collections::BTreeSet;

use crate::shared::local_date::LocalDate;

/// The days a habit was marked done, kept forever. One-completion-per-day is
/// structural (a set cannot hold a duplicate date), never a runtime guard.
/// Ordered so the calendar dots read chronologically.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CompletionHistory(BTreeSet<LocalDate>);

impl CompletionHistory {
    pub fn new() -> CompletionHistory {
        CompletionHistory::default()
    }

    /// Marks `day` done if it is not, clears it if it already is.
    pub fn toggle(&mut self, day: LocalDate) {
        if !self.0.remove(&day) {
            self.0.insert(day);
        }
    }

    pub fn contains(&self, day: LocalDate) -> bool {
        self.0.contains(&day)
    }

    /// The completed days within `from..=to`, inclusive on both ends. Walks
    /// the recorded completions rather than the calendar, so the cost is
    /// bounded by how much was actually practised, never by the span between
    /// the two dates.
    pub fn between(&self, from: LocalDate, to: LocalDate) -> impl Iterator<Item = LocalDate> + '_ {
        self.0.range(from..=to).copied()
    }
}
