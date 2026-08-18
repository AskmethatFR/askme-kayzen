use std::rc::Rc;

use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::habit_management::domain::lifecycle_state::LifecycleState;

/// The Ancrées screen's per-screen read model (adr-0006): just enough to name
/// each anchored habit and to state how many habits the daily life holds in
/// parallel — no Clock, nothing dated is shown (C4).
#[derive(Debug, Clone, PartialEq)]
pub struct AnchoredHabit {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnchoredScreen {
    pub habits: Vec<AnchoredHabit>,
    pub in_daily_life: usize,
}

#[derive(Clone)]
pub struct ListAnchoredHabits {
    repository: Rc<dyn HabitRepository>,
}

impl ListAnchoredHabits {
    pub fn new(repository: Rc<dyn HabitRepository>) -> ListAnchoredHabits {
        ListAnchoredHabits { repository }
    }

    pub fn handle(&self) -> AnchoredScreen {
        AnchoredScreen {
            habits: Vec::new(),
            in_daily_life: 0,
        }
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
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        let query = list_over(repository);

        let result = query.handle();

        assert_eq!(
            result,
            AnchoredScreen {
                habits: vec![AnchoredHabit {
                    id: "h-3".to_string(),
                    title: "Read one page".to_string(),
                }],
                in_daily_life: 2,
            }
        );
    }

    // @scenario: readmit-habit/S4
    #[test]
    fn in_daily_life_counts_every_non_anchored_habit_including_paused_ones() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", "Move a little"));
        repository.save(&a_habit("h-2", "Drink water"));
        let mut paused = a_habit("h-3", "Breathe");
        paused.pause().expect("a fresh habit is active");
        repository.save(&paused);
        let mut anchored_a = a_habit("h-4", "Read one page");
        anchored_a.anchor().expect("a fresh habit is active");
        repository.save(&anchored_a);
        let mut anchored_b = a_habit("h-5", "Write a line");
        anchored_b.anchor().expect("a fresh habit is active");
        repository.save(&anchored_b);
        let query = list_over(repository);

        let result = query.handle();

        assert_eq!(result.habits.len(), 2);
        assert_eq!(
            result.habits,
            vec![
                AnchoredHabit {
                    id: "h-4".to_string(),
                    title: "Read one page".to_string(),
                },
                AnchoredHabit {
                    id: "h-5".to_string(),
                    title: "Write a line".to_string(),
                },
            ]
        );
        assert_eq!(
            result.in_daily_life, 3,
            "a paused habit is still part of the daily life — only anchored ones are not"
        );
    }
}
