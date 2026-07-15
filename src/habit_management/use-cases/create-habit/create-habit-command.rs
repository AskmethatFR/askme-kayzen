use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::shared::guid_generator::GuidGenerator;
use std::error::Error;
use std::rc::Rc;

pub trait CreateHabitCommand {
    fn execute(&self, description: String, initial_duration: u32) -> Result<Habit, Box<dyn Error>>;
}

pub struct CreateHabit {
    guid_generator: Box<dyn GuidGenerator>,
    repository: Rc<dyn HabitRepository>,
}

impl CreateHabit {
    pub fn new(
        guid_generator: Box<dyn GuidGenerator>,
        repository: Rc<dyn HabitRepository>,
    ) -> CreateHabit {
        CreateHabit {
            guid_generator,
            repository,
        }
    }
}

impl CreateHabitCommand for CreateHabit {
    fn execute(&self, description: String, initial_duration: u32) -> Result<Habit, Box<dyn Error>> {
        let id = self.guid_generator.generate();
        let habit = Habit::new(id, description, initial_duration)?;
        self.repository.save(&habit);
        Ok(habit)
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateHabit, CreateHabitCommand};
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_repository::HabitRepository;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::shared::guid_generator::GuidGenerator;
    use std::rc::Rc;

    struct StubGuidGenerator {
        guid: String,
    }

    impl GuidGenerator for StubGuidGenerator {
        fn generate(&self) -> String {
            self.guid.clone()
        }
    }

    fn command_with(repository: Rc<InMemoryHabitRepository>) -> CreateHabit {
        let generator = StubGuidGenerator {
            guid: String::from("fixed-guid"),
        };
        CreateHabit::new(Box::new(generator), repository)
    }

    #[test]
    fn create_habit_persists_it() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let command = command_with(Rc::clone(&repository));

        let result = command
            .execute(String::from("My first habit"), 2)
            .expect("habit should be created");

        let expected = Habit::new(
            String::from("fixed-guid"),
            String::from("My first habit"),
            2,
        )
        .unwrap();
        assert_eq!(result, expected);
        assert_eq!(repository.all(), vec![expected]);
    }

    #[test]
    fn create_easy_habit_no_more_than_five_minutes() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let command = command_with(Rc::clone(&repository));

        let result = command.execute(String::from("Run a marathon"), 6);

        assert!(result.is_err());
        assert!(repository.all().is_empty());
    }

    #[test]
    fn create_easy_habit_of_exactly_five_minutes() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let command = command_with(Rc::clone(&repository));

        let result = command.execute(String::from("Read one page"), 5);

        assert!(result.is_ok());
    }

    #[test]
    fn create_habit_without_description() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let command = command_with(Rc::clone(&repository));

        let result = command.execute(String::from(""), 5);

        assert!(result.is_err());
        assert!(repository.all().is_empty());
    }

    #[test]
    fn create_habit_with_description_too_big() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let command = command_with(Rc::clone(&repository));

        let result = command.execute("a".repeat(51), 5);

        assert!(result.is_err());
        assert!(repository.all().is_empty());
    }
}
