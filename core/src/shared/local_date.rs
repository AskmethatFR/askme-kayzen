/// A calendar day in the user's local timezone, library-free by design: the
/// domain never sees `chrono`. Internally an epoch-day integer (signed days
/// since a fixed epoch) so future window arithmetic ("last 14 days") is plain
/// integer math, with no hand-rolled calendar/leap-year logic in the domain.
/// Conversion to/from a real calendar date lives in the infra `Clock` adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDate(i64);

impl LocalDate {
    pub fn from_epoch_day(day: i64) -> LocalDate {
        LocalDate(day)
    }

    /// The day `days` before this one. Kept inside the type rather than exposed
    /// as an epoch-day accessor: a window is calendar arithmetic, and handing
    /// callers the raw integer invites them to do that arithmetic themselves.
    pub fn minus_days(self, days: i64) -> LocalDate {
        LocalDate(self.0 - days)
    }

    /// The raw epoch-day integer, for the persistence codec's serialized form
    /// only. `pub(crate)`, not `pub`: `kayzen-app` still never sees the
    /// representation, so this does not reopen the arithmetic concern above.
    pub(crate) fn epoch_day(&self) -> i64 {
        self.0
    }
}
