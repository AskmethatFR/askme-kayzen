use std::rc::Rc;

use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;

#[derive(Clone)]
pub struct GetHabitDetail {
    repository: Rc<dyn HabitRepository>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HabitDetail {
    pub id: String,
    pub title: String,
    pub current_goal: u32,
    pub steps: Vec<u32>,
    pub next_goal_up: u32,
    pub next_goal_down: u32,
}

impl GetHabitDetail {
    pub fn new(repository: Rc<dyn HabitRepository>) -> GetHabitDetail {
        GetHabitDetail { repository }
    }

    pub fn handle(&self, habit_id: &str) -> Option<HabitDetail> {
        let id = HabitId::new(habit_id).ok()?;
        let habit = self.repository.get(&id)?;

        Some(HabitDetail {
            id: habit.id().value().to_string(),
            title: habit.title().value().to_string(),
            current_goal: habit.current_goal(),
            steps: habit
                .step_history()
                .changes()
                .into_iter()
                .map(|step| step.goal().value())
                .collect(),
            next_goal_up: habit.step_history().current().grown().value(),
            next_goal_down: habit.current_goal(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{GetHabitDetail, HabitDetail};
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::habit_repository::HabitRepository;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::shared::local_date::LocalDate;
    use std::rc::Rc;

    const CREATED_ON: i64 = 20_000;

    fn a_habit() -> Habit {
        Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn get_habit_detail_over(repository: Rc<InMemoryHabitRepository>) -> GetHabitDetail {
        GetHabitDetail::new(repository as Rc<dyn HabitRepository>)
    }

    // Test List — GetHabitDetail (@feature:adjust-goal, staircase display — the
    // scenario gap noted under "Open questions"):
    // - a known habit id returns its detail: title, current goal, one-step staircase.
    // - an unknown habit id returns None (stale URL / deleted habit, d2).
    #[test]
    fn a_known_habit_returns_its_title_goal_and_staircase() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        let query = get_habit_detail_over(Rc::clone(&repository));

        let result = query.handle("h-1");

        assert_eq!(
            result,
            Some(HabitDetail {
                id: "h-1".to_string(),
                title: "Read one page".to_string(),
                current_goal: 5,
                steps: vec![5],
                next_goal_up: 6,
                next_goal_down: 4,
            })
        );
    }

    // No Gherkin scenario names this path yet (next_goal_down at the floor) —
    // flagged under "Open questions".
    #[test]
    fn a_habit_at_the_floor_offers_lightening_toward_the_same_floor() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(1).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        ));
        let query = get_habit_detail_over(Rc::clone(&repository));

        let result = query.handle("h-1");

        assert_eq!(result.map(|detail| detail.next_goal_down), Some(1));
    }

    // No Gherkin scenario names this path yet (d2, stale URL / deleted habit) —
    // flagged under "Open questions" rather than authoring one (PM's lane).
    #[test]
    fn an_unknown_habit_id_returns_none() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let query = get_habit_detail_over(repository);

        assert_eq!(query.handle("missing"), None);
    }

    // No Gherkin scenario names this path yet either (invalid-id refusal,
    // T1 conformance with adr-0001) — flagged under "Open questions".
    #[test]
    fn an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        let query = get_habit_detail_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        assert_eq!(query.handle(&too_long), None);
    }
}
