use std::rc::Rc;

use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::habit_management::domain::lifecycle_state::LifecycleState;
use crate::shared::clock::Clock;

#[derive(Clone)]
pub struct ListBoardHabits {
    repository: Rc<dyn HabitRepository>,
    clock: Rc<dyn Clock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HabitSummary {
    pub id: String,
    pub title: String,
    pub minutes: u32,
    pub done_today: bool,
}

/// A paused habit's presence in the Today screen's paused zone — just enough
/// to name it and let the user reach its detail; no goal, no completion
/// status, because a pause carries no daily pressure (adr-0007 AD-2).
#[derive(Debug, Clone, PartialEq)]
pub struct PausedHabit {
    pub id: String,
    pub title: String,
}

/// The Today screen's per-screen read model (adr-0006): active habits carry
/// the daily pressure, paused ones sit in their own zone with none. The split
/// is the query's job, not the view's, so the tally over `active` is correct
/// by construction (adr-0007 AD-1).
#[derive(Debug, Clone, PartialEq)]
pub struct TodayHabits {
    pub active: Vec<HabitSummary>,
    pub paused: Vec<PausedHabit>,
    pub anchored_count: usize,
}

impl ListBoardHabits {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> Self {
        ListBoardHabits { repository, clock }
    }

    pub fn handle(&self) -> TodayHabits {
        let today = self.clock.today();
        let mut active = Vec::new();
        let mut paused = Vec::new();
        let mut anchored_count = 0;

        for habit in self.repository.all() {
            match habit.state() {
                LifecycleState::Active => active.push(HabitSummary {
                    id: habit.id().value().to_string(),
                    title: habit.title().value().to_string(),
                    minutes: habit.current_goal(),
                    done_today: habit.is_done_on(today),
                }),
                LifecycleState::Paused => paused.push(PausedHabit {
                    id: habit.id().value().to_string(),
                    title: habit.title().value().to_string(),
                }),
                LifecycleState::Anchored => {}
            }
        }

        TodayHabits {
            active,
            paused,
            anchored_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HabitSummary, ListBoardHabits, PausedHabit};
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

    fn a_habit() -> Habit {
        Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(3).unwrap(),
            LocalDate::from_epoch_day(TODAY),
        )
    }

    fn list_over(repository: Rc<InMemoryHabitRepository>) -> ListBoardHabits {
        ListBoardHabits::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(TODAY))) as Rc<dyn Clock>,
        )
    }

    // @scenario: today-habit-list/S1
    #[test]
    fn no_habits_yields_no_summaries() {
        let repository = Rc::new(InMemoryHabitRepository::new());

        let query = list_over(repository);

        let result = query.handle();

        assert_eq!(result.active, Vec::new());
        assert_eq!(result.paused, Vec::new());
    }

    // @scenario: today-habit-list/S2
    #[test]
    fn a_habit_maps_to_its_summary_with_honest_defaults() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        let query = list_over(Rc::clone(&repository));

        let result = query.handle();

        assert_eq!(
            result.active,
            vec![HabitSummary {
                id: "h-1".to_string(),
                title: "Read one page".to_string(),
                minutes: 3,
                done_today: false,
            }]
        );
    }

    // @scenario: today-habit-list/S3
    #[test]
    fn a_habit_done_today_is_reported_done() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.toggle_done(LocalDate::from_epoch_day(TODAY));
        repository.save(&habit);
        let query = list_over(Rc::clone(&repository));

        let result = query.handle();

        assert!(result.active[0].done_today);
    }

    // No Gherkin scenario names this path yet (a completion belongs to its own
    // day only) — flagged under "Open questions". Moved up from a Habit unit
    // test on PR #1 review: only use-case and service tests may pin a domain
    // principle.
    #[test]
    fn a_habit_done_on_another_day_is_not_reported_done_today() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.toggle_done(LocalDate::from_epoch_day(TODAY - 1));
        repository.save(&habit);
        let query = list_over(Rc::clone(&repository));

        let result = query.handle();

        assert!(!result.active[0].done_today);
    }

    // @scenario: pause-resume/S1
    #[test]
    fn a_paused_habit_is_absent_from_active_and_present_in_paused() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut paused = a_habit();
        paused.pause();
        repository.save(&paused);
        let active = Habit::new(
            HabitId::new("h-2").unwrap(),
            HabitTitle::new("Move a little".to_string()).unwrap(),
            Goal::new(4).unwrap(),
            LocalDate::from_epoch_day(TODAY),
        );
        repository.save(&active);
        let query = list_over(Rc::clone(&repository));

        let result = query.handle();

        assert_eq!(
            result.active,
            vec![HabitSummary {
                id: "h-2".to_string(),
                title: "Move a little".to_string(),
                minutes: 4,
                done_today: false,
            }]
        );
        assert_eq!(
            result.paused,
            vec![PausedHabit {
                id: "h-1".to_string(),
                title: "Read one page".to_string(),
            }]
        );
    }

    // @scenario: anchor-habit/S2
    #[test]
    fn an_anchored_habit_is_absent_from_both_active_and_paused() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut anchored = a_habit();
        anchored.anchor();
        repository.save(&anchored);
        let active = Habit::new(
            HabitId::new("h-2").unwrap(),
            HabitTitle::new("Move a little".to_string()).unwrap(),
            Goal::new(4).unwrap(),
            LocalDate::from_epoch_day(TODAY),
        );
        repository.save(&active);
        let query = list_over(Rc::clone(&repository));

        let result = query.handle();

        assert_eq!(
            result.active,
            vec![HabitSummary {
                id: "h-2".to_string(),
                title: "Move a little".to_string(),
                minutes: 4,
                done_today: false,
            }]
        );
        assert_eq!(result.paused, Vec::new());
    }

    // @scenario: anchor-habit/S2
    #[test]
    fn anchored_habits_are_counted() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut first_anchored = a_habit();
        first_anchored.anchor();
        repository.save(&first_anchored);
        let mut second_anchored = Habit::new(
            HabitId::new("h-2").unwrap(),
            HabitTitle::new("Move a little".to_string()).unwrap(),
            Goal::new(4).unwrap(),
            LocalDate::from_epoch_day(TODAY),
        );
        second_anchored.anchor();
        repository.save(&second_anchored);
        let query = list_over(Rc::clone(&repository));

        let result = query.handle();

        assert_eq!(result.anchored_count, 2);
    }

    // @scenario: pause-resume/S2
    #[test]
    fn a_resumed_habit_reappears_in_active_and_leaves_paused() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.pause();
        habit.resume();
        repository.save(&habit);
        let query = list_over(Rc::clone(&repository));

        let result = query.handle();

        assert_eq!(
            result.active,
            vec![HabitSummary {
                id: "h-1".to_string(),
                title: "Read one page".to_string(),
                minutes: 3,
                done_today: false,
            }]
        );
        assert_eq!(result.paused, Vec::new());
    }
}
