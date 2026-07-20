use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_repository::HabitRepository;
use std::rc::Rc;

#[derive(Clone)]
pub struct ListBoardHabits {
    repository: Rc<dyn HabitRepository>,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub struct HabitSummary {
    pub id: String,       // depuis HabitId
    pub title: String,    // depuis HabitTitle
    pub minutes: u32,     // depuis current_dose()
    pub done_today: bool, // false pour l'instant
}

impl From<&Habit> for HabitSummary {
    fn from(habit: &Habit) -> Self {
        HabitSummary {
            id: habit.id().value().to_string(),
            title: habit.title().value().to_string(),
            minutes: habit.current_dose(),
            done_today: false,
        }
    }
}

impl ListBoardHabits {
    pub fn new(repository: Rc<dyn HabitRepository>) -> Self {
        ListBoardHabits { repository }
    }

    pub fn handle(&self) -> Vec<HabitSummary> {
        self.repository.all().iter().map(HabitSummary::from).collect()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::habit_repository::HabitRepository;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::domain::initial_duration::InitialDuration;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::habit_management::queries::list_board_habits::list_board_habits::{
        HabitSummary, ListBoardHabits,
    };
    use std::rc::Rc;

    #[test]
    fn read_habit_board_when_no_habits() {
        let repository = Rc::new(InMemoryHabitRepository::new());

        let query = ListBoardHabits::new(repository);

        let result = query.handle();
        let expected: Vec<HabitSummary> = Vec::new();

        assert_eq!(result, expected);
    }

    #[test]
    fn read_habit_board_when_one_habit() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());

        let query = ListBoardHabits::new(repository);
        let result = query.handle();
        let expected = vec![HabitSummary {
            id: "my_id".to_string(),
            title: "my title".to_string(),
            minutes: 3,
            done_today: false,
        }];

        assert_eq!(result, expected);
    }

    fn a_habit() -> Habit {
        let id = HabitId::new("my_id".to_string());
        let title = HabitTitle::new("my title".to_string()).unwrap();
        let initial_duration = InitialDuration::new(3).unwrap();

        let habit = Habit::new(id, title, initial_duration);
        return habit;
    }
}
