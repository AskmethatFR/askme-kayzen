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
}
