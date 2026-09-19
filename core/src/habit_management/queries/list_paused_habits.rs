use std::rc::Rc;

use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::habit_management::domain::lifecycle_state::LifecycleState;
use crate::habit_management::queries::list_board_habits::PausedHabit;

#[derive(Clone)]
pub struct ListPausedHabits {
    repository: Rc<dyn HabitRepository>,
}

impl ListPausedHabits {
    pub fn new(repository: Rc<dyn HabitRepository>) -> ListPausedHabits {
        ListPausedHabits { repository }
    }

    pub fn handle(&self) -> Vec<PausedHabit> {
        self.repository
            .all()
            .into_iter()
            .filter(|habit| habit.state() == LifecycleState::Paused)
            .map(|habit| PausedHabit {
                id: habit.id().value().to_string(),
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

    fn list_over(repository: Rc<InMemoryHabitRepository>) -> ListPausedHabits {
        ListPausedHabits::new(repository as Rc<dyn HabitRepository>)
    }

    #[test]
    fn an_empty_board_yields_no_paused_habits() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let query = list_over(repository);

        let result = query.handle();

        assert_eq!(result, Vec::new());
    }

    #[test]
    fn only_paused_habits_are_listed() {
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
            vec![PausedHabit {
                id: "h-2".to_string(),
                title: "Breathe".to_string(),
            }]
        );
    }

    #[test]
    fn a_board_of_only_paused_habits_lists_them_all() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut first = a_habit("h-1", "Breathe");
        first.pause().expect("a fresh habit is active");
        repository.save(&first);
        let mut second = a_habit("h-2", "Write a line");
        second.pause().expect("a fresh habit is active");
        repository.save(&second);
        let query = list_over(repository);

        let result = query.handle();

        assert_eq!(
            result,
            vec![
                PausedHabit {
                    id: "h-1".to_string(),
                    title: "Breathe".to_string(),
                },
                PausedHabit {
                    id: "h-2".to_string(),
                    title: "Write a line".to_string(),
                },
            ]
        );
    }
}
