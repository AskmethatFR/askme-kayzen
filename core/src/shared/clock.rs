use crate::shared::local_date::LocalDate;

/// Port giving the domain "today" as a library-free `LocalDate`. The use case
/// holds a `Clock` and passes `clock.today()` into aggregate methods, so the
/// aggregate stays a pure function of its inputs.
pub trait Clock {
    fn today(&self) -> LocalDate;
}

/// The calendar parts of a day, for the surfaces that must name it: weekday
/// 0=Monday…6=Sunday, day 1–31, month 1–12 — numbers only, so every locale's
/// words stay in the app's Fluent catalogue (adr-0018).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarParts {
    pub weekday: u32,
    pub day: u32,
    pub month: u32,
}

/// Pure conversion from the domain's epoch-day `LocalDate` to its calendar
/// parts. Lives here because `LocalDate` is library-free by design (adr-0007)
/// and this file already is chrono's only home, on the same
/// `num_days_from_ce` convention `SystemClock` writes.
pub fn calendar_parts(date: LocalDate) -> CalendarParts {
    use chrono::Datelike;
    let day = chrono::NaiveDate::from_num_days_from_ce_opt(
        i32::try_from(date.epoch_day()).expect("an epoch day fits in i32"),
    )
    .expect("a LocalDate always names a real calendar day");
    CalendarParts {
        weekday: day.weekday().num_days_from_monday(),
        day: day.day(),
        month: day.month(),
    }
}

/// Real adapter: the only place `chrono` touches the crate. Converts the local
/// calendar date to the domain's epoch-day `LocalDate` at the boundary.
pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> LocalDate {
        use chrono::Datelike;
        let today = chrono::Local::now().date_naive();
        LocalDate::from_epoch_day(i64::from(today.num_days_from_ce()))
    }
}

#[cfg(test)]
pub(crate) struct FixedClock {
    today: LocalDate,
}

#[cfg(test)]
impl FixedClock {
    pub(crate) fn new(today: LocalDate) -> FixedClock {
        FixedClock { today }
    }
}

#[cfg(test)]
impl Clock for FixedClock {
    fn today(&self) -> LocalDate {
        self.today
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_parts_reads_the_epoch_day_convention_the_clock_writes() {
        assert_eq!(
            calendar_parts(LocalDate::from_epoch_day(739_878)),
            CalendarParts {
                weekday: 5,
                day: 19,
                month: 9,
            },
            "expected epoch day 739_878 to be Saturday 19 September 2026"
        );
    }

    #[test]
    fn calendar_parts_reads_the_epoch_day_convention_at_its_origin() {
        assert_eq!(
            calendar_parts(LocalDate::from_epoch_day(719_163)),
            CalendarParts {
                weekday: 3,
                day: 1,
                month: 1,
            },
            "expected epoch day 719_163 to be Thursday 1 January 1970"
        );
    }
}
