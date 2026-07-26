use std::rc::Rc;

use kayzen_core::habit_management::domain::domain_event_publisher::DomainEventPublisher;
use kayzen_core::habit_management::domain::habit_board::HabitBoardError;
use kayzen_core::habit_management::domain::habit_board_repository::HabitBoardRepository;
use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
use kayzen_core::habit_management::infrastructure::in_memory_outbox::InMemoryOutbox;
use kayzen_core::habit_management::use_cases::create_habit_on_request::CreateHabitOnRequest;
use kayzen_core::habit_management::use_cases::request_habit::RequestHabit;
use kayzen_core::shared::clock::Clock;
use kayzen_core::shared::guid_generator::UuidGenerator;

/// The default daily goal offered to every new habit — a flexible target,
/// not a ceiling. Kaizen begins gently, not necessarily tiny.
const STARTING_GOAL: u32 = 5;

/// App service that adds a habit end to end: request it on the board, then drain
/// the outbox and let the create handler persist it. This is the composition
/// root's synchronous outbox dispatcher, isolated in its own single-responsibility
/// type rather than piled onto the Services registry. Surfaces the board's refusal
/// (full / duplicate / invalid title) to the caller.
#[derive(Clone)]
pub struct AddHabit {
    habit_repository: Rc<dyn HabitRepository>,
    board_repository: Rc<dyn HabitBoardRepository>,
    outbox: Rc<InMemoryOutbox>,
    clock: Rc<dyn Clock>,
}

impl AddHabit {
    pub fn new(
        habit_repository: Rc<dyn HabitRepository>,
        board_repository: Rc<dyn HabitBoardRepository>,
        outbox: Rc<InMemoryOutbox>,
        clock: Rc<dyn Clock>,
    ) -> AddHabit {
        AddHabit {
            habit_repository,
            board_repository,
            outbox,
            clock,
        }
    }

    pub fn execute(&self, title: &str) -> Result<(), HabitBoardError> {
        let request_habit = RequestHabit::new(
            Box::new(UuidGenerator),
            Rc::clone(&self.outbox) as Rc<dyn DomainEventPublisher>,
            Rc::clone(&self.board_repository),
        );
        request_habit.execute(title.to_string(), STARTING_GOAL)?;

        let create_habit =
            CreateHabitOnRequest::new(Rc::clone(&self.habit_repository), Rc::clone(&self.clock));
        for event in self.outbox.drain() {
            create_habit.handle(event);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_board_repository::InMemoryHabitBoardRepository;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use kayzen_core::shared::clock::SystemClock;

    fn an_add_habit(habit_repository: Rc<dyn HabitRepository>) -> AddHabit {
        AddHabit::new(
            habit_repository,
            Rc::new(InMemoryHabitBoardRepository::new()),
            Rc::new(InMemoryOutbox::new()),
            Rc::new(SystemClock),
        )
    }

    #[test]
    fn adding_a_habit_persists_it_on_the_board() {
        let habit_repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let add_habit = an_add_habit(Rc::clone(&habit_repository));

        add_habit.execute("Lire une page").unwrap();

        let titles: Vec<String> = habit_repository
            .all()
            .iter()
            .map(|habit| habit.title().value().to_string())
            .collect();
        assert!(titles.contains(&"Lire une page".to_string()));
    }
}
