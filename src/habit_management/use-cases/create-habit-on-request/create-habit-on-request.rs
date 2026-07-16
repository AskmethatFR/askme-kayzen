use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
use crate::habit_management::domain::habit_repository::HabitRepository;
use std::rc::Rc;

pub struct CreateHabitOnRequest {
    repository: Rc<dyn HabitRepository>,
}

impl CreateHabitOnRequest {
    pub fn new(repository: Rc<dyn HabitRepository>) -> CreateHabitOnRequest {
        CreateHabitOnRequest { repository }
    }

    pub fn handle(&self, event: HabitBoardEvent) {
        todo!("materialize the requested Habit and persist it via the repository")
    }
}

// Test List — CreateHabitOnRequest::handle
// - [T3] handling a HabitRequested event persists the corresponding Habit (same id,
//   description, duration) via HabitRepository (AC3), fed directly (replaces
//   create_habit_persists_it's persistence assertion).
// - [T3] end-to-end: RequestHabit::execute -> outbox.drain() (in the test) ->
//   handle() -> the repository contains the resulting Habit (AC3 full round trip,
//   replaces create_habit_persists_it entirely).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::domain_event_publisher::DomainEventPublisher;
    use crate::habit_management::domain::habit_description::HabitDescription;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::initial_duration::InitialDuration;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::habit_management::infrastructure::in_memory_outbox::InMemoryOutbox;
    use crate::habit_management::use_cases::request_habit::request_habit::RequestHabit;
    use crate::shared::guid_generator::GuidGenerator;

    struct StubGuidGenerator {
        guid: String,
    }

    impl GuidGenerator for StubGuidGenerator {
        fn generate(&self) -> String {
            self.guid.clone()
        }
    }

    fn a_habit(id: &str, description: &str, initial_duration: u32) -> Habit {
        Habit::new(
            HabitId::from(id),
            HabitDescription::new(String::from(description)).unwrap(),
            InitialDuration::new(initial_duration).unwrap(),
        )
    }

    fn a_habit_requested(id: &str, description: &str, initial_duration: u32) -> HabitBoardEvent {
        HabitBoardEvent::HabitRequested {
            id: HabitId::from(id),
            description: HabitDescription::new(String::from(description)).unwrap(),
            initial_duration: InitialDuration::new(initial_duration).unwrap(),
        }
    }

    #[test]
    fn handling_a_habit_requested_event_persists_the_habit() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let handler = CreateHabitOnRequest::new(Rc::clone(&repository) as Rc<dyn HabitRepository>);

        handler.handle(a_habit_requested("id-1", "Read one page", 2));

        assert_eq!(
            repository.all(),
            vec![a_habit("id-1", "Read one page", 2)]
        );
    }

    #[test]
    fn requesting_then_handling_a_habit_persists_it_end_to_end() {
        let outbox = Rc::new(InMemoryOutbox::new());
        let repository = Rc::new(InMemoryHabitRepository::new());
        let request_habit = RequestHabit::new(
            Box::new(StubGuidGenerator {
                guid: String::from("fixed-guid"),
            }),
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
        );
        let handler = CreateHabitOnRequest::new(Rc::clone(&repository) as Rc<dyn HabitRepository>);

        request_habit
            .execute(String::from("Read one page"), 2)
            .expect("valid habit request");

        for event in outbox.drain() {
            handler.handle(event);
        }

        assert_eq!(
            repository.all(),
            vec![a_habit("fixed-guid", "Read one page", 2)]
        );
    }
}
