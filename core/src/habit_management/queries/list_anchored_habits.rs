use std::rc::Rc;

use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::habit_management::domain::lifecycle_state::LifecycleState;

/// The Ancrées screen's per-screen read model (adr-0006): just enough to name
/// each anchored habit — no Clock, nothing dated is shown (C4).
#[derive(Debug, Clone, PartialEq)]
pub struct AnchoredHabit {
    pub title: String,
}

#[derive(Clone)]
pub struct ListAnchoredHabits {
    repository: Rc<dyn HabitRepository>,
}

impl ListAnchoredHabits {
    pub fn new(repository: Rc<dyn HabitRepository>) -> ListAnchoredHabits {
        ListAnchoredHabits { repository }
    }

    pub fn handle(&self) -> Vec<AnchoredHabit> {
        self.repository
            .all()
            .into_iter()
            .filter(|habit| habit.state() == LifecycleState::Anchored)
            .map(|habit| AnchoredHabit {
                title: habit.title().value().to_string(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::shared::local_date::LocalDate;

    const CREATED_ON: i64 = 19_990;

    fn a_habit(id: &str, title: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new(title.to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn list_over(repository: Rc<InMemoryHabitRepository>) -> ListAnchoredHabits {
        ListAnchoredHabits::new(repository as Rc<dyn HabitRepository>)
    }

    // @scenario: anchor-habit/S2
    #[test]
    fn only_anchored_habits_are_listed() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", "Move a little"));
        let mut paused = a_habit("h-2", "Breathe");
        paused.pause().expect("a fresh habit is active");
        repository.save(&paused);
        let mut anchored = a_habit("h-3", "Read one page");
        anchored.anchor();
        repository.save(&anchored);
        let query = list_over(repository);

        let result = query.handle();

        assert_eq!(
            result,
            vec![AnchoredHabit {
                title: "Read one page".to_string(),
            }]
        );
    }
}
