use crate::habit_management::domain::domain_event_publisher::DomainEventPublisher;
use crate::habit_management::domain::habit::HabitError;
use crate::habit_management::domain::habit_board::HabitBoard;
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

    pub fn execute(&self, title: String, initial_duration: u32) -> Result<HabitId, HabitError> {
        let id = HabitId::new(self.guid_generator.generate());
        let event = HabitBoard::new().request_habit(id.clone(), title, initial_duration)?;
        self.publisher.publish(event);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::domain::initial_duration::InitialDuration;
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
        let cases = vec![
            (String::from("Read one page"), 5),
            ("a".repeat(HabitTitle::MIN_LEN), 5),
            ("a".repeat(HabitTitle::MAX_LEN), 5),
        ];

        for (title, initial_duration) in cases {
            let outbox = Rc::new(InMemoryOutbox::new());
            let request_habit = request_habit_with(Rc::clone(&outbox));

            let result = request_habit.execute(title.clone(), initial_duration);

            assert_eq!(result, Ok(HabitId::from("fixed-guid")));
            assert_eq!(
                outbox.drain(),
                vec![HabitBoardEvent::HabitRequested {
                    id: HabitId::from("fixed-guid"),
                    title: HabitTitle::new(title).unwrap(),
                    initial_duration: InitialDuration::new(initial_duration).unwrap(),
                }]
            );
        }
    }

    #[test]
    fn requesting_an_invalid_habit_returns_an_error_and_publishes_nothing() {
        let cases: Vec<(String, u32, HabitError)> = vec![
            (
                String::from("Run a marathon"),
                InitialDuration::MAX + 1,
                HabitError::DurationTooLong {
                    max: InitialDuration::MAX,
                },
            ),
            (
                String::new(),
                5,
                HabitError::TitleLength {
                    min: HabitTitle::MIN_LEN,
                    max: HabitTitle::MAX_LEN,
                },
            ),
            (
                "a".repeat(HabitTitle::MAX_LEN + 1),
                5,
                HabitError::TitleLength {
                    min: HabitTitle::MIN_LEN,
                    max: HabitTitle::MAX_LEN,
                },
            ),
        ];

        for (title, initial_duration, expected_error) in cases {
            let outbox = Rc::new(InMemoryOutbox::new());
            let request_habit = request_habit_with(Rc::clone(&outbox));

            let result = request_habit.execute(title, initial_duration);

            assert_eq!(result, Err(expected_error));
            assert!(outbox.drain().is_empty());
        }
    }
}
