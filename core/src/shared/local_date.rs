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

    /// The day `days` before this one. `saturating_sub`, not a bare `-`:
    /// defence in depth for a `LocalDate` built from `parse_stored`'s far
    /// end of the accepted range, where a bare subtraction could still
    /// overflow depending on `days`.
    pub fn minus_days(self, days: i64) -> LocalDate {
        LocalDate(self.0.saturating_sub(days))
    }

    /// The raw epoch-day integer, for the persistence codec's serialized form
    /// only. `pub(crate)`, not `pub`: `kayzen-app` still never sees the
    /// representation, so this does not reopen the arithmetic concern above.
    pub(crate) fn epoch_day(self) -> i64 {
        self.0
    }

    /// The only validating constructor for a date coming from outside this
    /// process — a stored snapshot payload. `from_epoch_day` stays total and
    /// unbounded for in-process callers (`Clock` adapters, tests), which
    /// only ever produce values near today; this is adr-0001 applied to the
    /// one type that lacked it, closing Security's F-1 (a persistent
    /// unbounded value here made `minutes_practised`'s day-by-day walk
    /// attacker-controlled). `0..=MAX_STORED_EPOCH_DAY`: today's
    /// `num_days_from_ce()` is ~739_000; 4_000_000 days is roughly ten
    /// millennia of headroom, comfortably bounding the walk without ever
    /// rejecting a real calendar date this app will see.
    pub(crate) fn parse_stored(day: i64) -> Option<LocalDate> {
        if (0..=Self::MAX_STORED_EPOCH_DAY).contains(&day) {
            Some(LocalDate(day))
        } else {
            None
        }
    }

    const MAX_STORED_EPOCH_DAY: i64 = 4_000_000;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test List — LocalDate::parse_stored boundary (adr-0009: comparison
    // mutants on the bound die only on exact-boundary tests):
    // - values within [0, MAX_STORED_EPOCH_DAY] are accepted.
    // - values outside that range are rejected, including the extremes an
    //   attacker actually controls (i64::MIN, i64::MAX).

    #[test]
    fn parse_stored_accepts_values_within_the_bound() {
        let cases = vec![0, LocalDate::MAX_STORED_EPOCH_DAY];

        for day in cases {
            assert_eq!(
                LocalDate::parse_stored(day),
                Some(LocalDate(day)),
                "expected {day} to be accepted"
            );
        }
    }

    #[test]
    fn parse_stored_rejects_values_outside_the_bound() {
        let cases = vec![-1, LocalDate::MAX_STORED_EPOCH_DAY + 1, i64::MIN, i64::MAX];

        for day in cases {
            assert_eq!(
                LocalDate::parse_stored(day),
                None,
                "expected {day} to be rejected"
            );
        }
    }
}
