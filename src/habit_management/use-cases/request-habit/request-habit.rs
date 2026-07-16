use crate::habit_management::domain::domain_event_publisher::DomainEventPublisher;
use crate::habit_management::domain::habit_board::{HabitBoard, HabitBoardError};
use crate::habit_management::domain::habit_id::HabitId;
use crate::shared::guid_generator::GuidGenerator;
use std::rc::Rc;

pub struct RequestHabit {
    guid_generator: Box<dyn GuidGenerator>,
    publisher: Rc<dyn DomainEventPublisher>,
}

impl RequestHabit {
    pub fn new(
        guid_generator: Box<dyn GuidGenerator>,
        publisher: Rc<dyn DomainEventPublisher>,
    ) -> RequestHabit {
        RequestHabit {
            guid_generator,
            publisher,
        }
    }

    pub fn execute(
        &self,
        description: String,
        initial_duration: u32,
    ) -> Result<HabitId, HabitBoardError> {
        let id = HabitId::new(self.guid_generator.generate());
        let event = HabitBoard::new().request_habit(id.clone(), description, initial_duration)?;
        self.publisher.publish(event);
        Ok(id)
    }
}

// Test List — RequestHabit::execute
// - [T1] valid description + duration publishes exactly one HabitRequested carrying
//   the generated id, description and duration (AC1). Uses duration = 5 so this same
//   case also pins the inclusive upper boundary (replaces the old
//   create_easy_habit_of_exactly_five_minutes case).
// - [T2, next cycle] invalid inputs (duration > 5, empty description, description > 50
//   chars) return an error and publish nothing (AC2) — added once HabitDescription/
//   InitialDuration exist.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
    use crate::habit_management::infrastructure::in_memory_outbox::InMemoryOutbox;

    struct StubGuidGenerator {
        guid: String,
    }

    impl GuidGenerator for StubGuidGenerator {
        fn generate(&self) -> String {
            self.guid.clone()
        }
    }

    fn request_habit_with(publisher: Rc<InMemoryOutbox>) -> RequestHabit {
        RequestHabit::new(
            Box::new(StubGuidGenerator {
                guid: String::from("fixed-guid"),
            }),
            publisher,
        )
    }

    #[test]
    fn requesting_a_habit_publishes_exactly_one_habit_requested_event() {
        let outbox = Rc::new(InMemoryOutbox::new());
        let request_habit = request_habit_with(Rc::clone(&outbox));

        let result = request_habit.execute(String::from("Read one page"), 5);

        assert_eq!(result, Ok(HabitId::from("fixed-guid")));
        assert_eq!(
            outbox.drain(),
            vec![HabitBoardEvent::HabitRequested {
                id: HabitId::from("fixed-guid"),
                description: String::from("Read one page"),
                initial_duration: 5,
            }]
        );
    }
}
