use std::rc::Rc;

use crate::habit_management::domain::habit_repository::HabitRepository;
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

impl ListBoardHabits {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> Self {
        ListBoardHabits { repository, clock }
    }

    pub fn handle(&self) -> Vec<HabitSummary> {
        let today = self.clock.today();

        self.repository
            .all()
            .iter()
            .map(|habit| HabitSummary {
                id: habit.id().value().to_string(),
                title: habit.title().value().to_string(),
                minutes: habit.current_goal(),
                done_today: habit.is_done_on(today),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{HabitSummary, ListBoardHabits};
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

        assert_eq!(query.handle(), Vec::new());
    }

    // @scenario: today-habit-list/S2
    #[test]
    fn a_habit_maps_to_its_summary_with_honest_defaults() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        let query = list_over(Rc::clone(&repository));

        let result = query.handle();

        assert_eq!(
            result,
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

        assert!(result[0].done_today);
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

        assert!(!result[0].done_today);
    }
}
