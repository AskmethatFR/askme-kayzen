use crate::shared::local_date::LocalDate;

/// Port giving the domain "today" as a library-free `LocalDate`. The use case
/// holds a `Clock` and passes `clock.today()` into aggregate methods, so the
/// aggregate stays a pure function of its inputs.
pub trait Clock {
    fn today(&self) -> LocalDate;
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
