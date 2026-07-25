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
        match event {
            HabitBoardEvent::HabitRequested { id, title, goal } => {
                let habit = Habit::new(id, title, goal);
                self.repository.save(&habit);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::domain_event_publisher::DomainEventPublisher;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit_board_repository::HabitBoardRepository;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::infrastructure::in_memory_habit_board_repository::InMemoryHabitBoardRepository;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::habit_management::infrastructure::in_memory_outbox::InMemoryOutbox;
    use crate::habit_management::use_cases::request_habit::RequestHabit;
    use crate::shared::guid_generator::GuidGenerator;

    struct StubGuidGenerator {
        guid: String,
    }

    impl GuidGenerator for StubGuidGenerator {
        fn generate(&self) -> String {
            self.guid.clone()
        }
    }

    fn a_habit(id: &str, title: &str, goal: u32) -> Habit {
        Habit::new(
            HabitId::from(id),
            HabitTitle::new(String::from(title)).unwrap(),
            Goal::new(goal).unwrap(),
        )
    }

    fn a_habit_requested(id: &str, title: &str, goal: u32) -> HabitBoardEvent {
        HabitBoardEvent::HabitRequested {
            id: HabitId::from(id),
            title: HabitTitle::new(String::from(title)).unwrap(),
            goal: Goal::new(goal).unwrap(),
        }
    }

    // @scenario: create-habit-on-request/S1
    #[test]
    fn handling_a_habit_requested_event_persists_the_habit() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let handler = CreateHabitOnRequest::new(Rc::clone(&repository) as Rc<dyn HabitRepository>);

        handler.handle(a_habit_requested("id-1", "Read one page", 2));

        assert_eq!(repository.all(), vec![a_habit("id-1", "Read one page", 2)]);
    }

    // @scenario: create-habit-on-request/S2
    #[test]
    fn requesting_then_handling_a_habit_persists_it_end_to_end() {
        let outbox = Rc::new(InMemoryOutbox::new());
        let repository = Rc::new(InMemoryHabitRepository::new());
        let board_repository = Rc::new(InMemoryHabitBoardRepository::new());
        let request_habit = RequestHabit::new(
            Box::new(StubGuidGenerator {
                guid: String::from("fixed-guid"),
            }),
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
            board_repository as Rc<dyn HabitBoardRepository>,
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
