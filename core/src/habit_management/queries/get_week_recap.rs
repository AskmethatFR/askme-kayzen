use std::rc::Rc;

use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::shared::clock::Clock;
use crate::shared::local_date::LocalDate;

/// How many trailing days count as "recently" for the week's word (adr-0006:
/// a rolling window ending today, never a Monday→Sunday calendar week).
const RECENT_PRACTICE_WINDOW_DAYS: i64 = 7;

#[derive(Clone)]
pub struct GetWeekRecap {
    repository: Rc<dyn HabitRepository>,
    clock: Rc<dyn Clock>,
}

/// The Week screen's per-screen read model (adr-0006): everything derived on
/// read from every habit's histories, whatever its lifecycle state (AD-3).
#[derive(Debug, Clone, PartialEq)]
pub struct WeekRecap {
    pub minutes_practised: u32,
    pub message: WeekMessage,
}

/// How the week is living right now, as the recap says it — a DTO-side enum
/// (adr-0006: a domain-shaped choice crosses as an enum, never a bool, and the
/// French words live in the view, never in core).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeekMessage {
    FreshStart,
    Resting,
    Growing,
}

impl GetWeekRecap {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> GetWeekRecap {
        GetWeekRecap { repository, clock }
    }

    pub fn handle(&self) -> WeekRecap {
        let today = self.clock.today();
        let mut minutes_practised = 0u32;
        let mut recently = false;

        for habit in self.repository.all() {
            minutes_practised = minutes_practised.saturating_add(habit.minutes_practised(today));
            if practised_recently(&habit, today) {
                recently = true;
            }
        }

        WeekRecap {
            minutes_practised,
            message: message_for(minutes_practised, recently),
        }
    }
}

fn practised_recently(habit: &Habit, today: LocalDate) -> bool {
    (0..RECENT_PRACTICE_WINDOW_DAYS).any(|days_back| habit.is_done_on(today.minus_days(days_back)))
}

fn message_for(minutes_practised: u32, practised_recently: bool) -> WeekMessage {
    if minutes_practised == 0 {
        return WeekMessage::FreshStart;
    }
    if practised_recently {
        return WeekMessage::Growing;
    }
    WeekMessage::Resting
}

#[cfg(test)]
mod tests {
    use super::{GetWeekRecap, WeekMessage};
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::habit_repository::HabitRepository;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::shared::clock::{Clock, FixedClock};
    use crate::shared::local_date::LocalDate;
    use std::rc::Rc;

    const TODAY: i64 = 20_000;

    fn a_habit(id: &str, goal: u32, created_on: i64) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(goal).unwrap(),
            LocalDate::from_epoch_day(created_on),
        )
    }

    fn get_week_recap_over(repository: Rc<InMemoryHabitRepository>) -> GetWeekRecap {
        GetWeekRecap::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(TODAY))) as Rc<dyn Clock>,
        )
    }

    // Test List — GetWeekRecap (@feature:week-recap):
    // - the figure sums minutes practised across every habit (S1).
    // - paused and anchored habits still count in the sum (S2, sum half only —
    //   the row-per-habit half of the same scenario belongs to task 3).
    // - no practice yet -> message FreshStart, figure at 0 (S3).
    // - practised earlier, not in the last 7 days -> message Resting (S4).
    // - practised within the last 7 days -> message Growing (branch
    //   completeness: no scenario names this arm on its own, it is the third
    //   arm the message rule already states in the tech spec).
    // - the running sum saturates across habits instead of overflowing.

    // @scenario: week-recap/S1
    #[test]
    fn the_figure_sums_minutes_practised_across_every_habit() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        for (id, days_done) in [("h-1", 3), ("h-2", 2), ("h-3", 1)] {
            let mut habit = a_habit(id, 5, TODAY - 6);
            for days_back in 0..days_done {
                habit.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
            }
            repository.save(&habit);
        }
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.minutes_practised, 30,
            "3 + 2 + 1 completed days at 5 minutes each sum to 30"
        );
    }

    // @scenario: week-recap/S2
    #[test]
    fn paused_and_anchored_habits_still_count_in_the_sum() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut paused = a_habit("h-1", 5, TODAY - 6);
        for days_back in 0..4 {
            paused.toggle_done(LocalDate::from_epoch_day(TODAY - 3 - days_back));
        }
        paused.pause().expect("a fresh habit is active");
        repository.save(&paused);

        let mut anchored = a_habit("h-2", 5, TODAY - 6);
        for days_back in 0..3 {
            anchored.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
        }
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);

        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.minutes_practised, 35,
            "4 days at 5 min from the paused habit plus 3 from the anchored \
             one — pausing or anchoring never takes lived minutes back"
        );
    }

    // @scenario: week-recap/S3
    #[test]
    fn a_week_with_no_practice_reads_as_a_fresh_start() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5, TODAY));
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.minutes_practised, 0);
        assert_eq!(recap.message, WeekMessage::FreshStart);
    }

    // @scenario: week-recap/S4
    #[test]
    fn practice_before_the_last_seven_days_is_acknowledged_as_rest() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 20);
        habit.toggle_done(LocalDate::from_epoch_day(TODAY - 10));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.minutes_practised, 5,
            "the lived minutes are never erased by rest"
        );
        assert_eq!(recap.message, WeekMessage::Resting);
    }

    #[test]
    fn practice_within_the_last_seven_days_reads_as_growing() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 2);
        habit.toggle_done(LocalDate::from_epoch_day(TODAY));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.message, WeekMessage::Growing);
    }

    #[test]
    fn the_running_sum_saturates_instead_of_overflowing_across_habits() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        for id in ["h-1", "h-2"] {
            let mut habit = a_habit(id, u32::MAX, TODAY);
            habit.toggle_done(LocalDate::from_epoch_day(TODAY));
            repository.save(&habit);
        }
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.minutes_practised, u32::MAX);
    }
}
