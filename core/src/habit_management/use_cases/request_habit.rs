use crate::habit_management::domain::domain_event_publisher::DomainEventPublisher;
use crate::habit_management::domain::habit_board::HabitBoardError;
use crate::habit_management::domain::habit_board_repository::HabitBoardRepository;
use crate::habit_management::domain::habit_id::HabitId;
use crate::shared::guid_generator::GuidGenerator;
use std::rc::Rc;

pub struct RequestHabit {
    guid_generator: Box<dyn GuidGenerator>,
    publisher: Rc<dyn DomainEventPublisher>,
    board_repository: Rc<dyn HabitBoardRepository>,
}

impl RequestHabit {
    pub fn new(
        guid_generator: Box<dyn GuidGenerator>,
        publisher: Rc<dyn DomainEventPublisher>,
        board_repository: Rc<dyn HabitBoardRepository>,
    ) -> RequestHabit {
        RequestHabit {
            guid_generator,
            publisher,
            board_repository,
        }
    }

    pub fn execute(&self, title: String, goal: u32) -> Result<HabitId, HabitBoardError> {
        let id =
            HabitId::new(&self.guid_generator.generate()).map_err(HabitBoardError::InvalidHabit)?;
        let mut board = self.board_repository.load();
        let event = board.request_habit(id.clone(), title, goal)?;
        self.board_repository.save(&board);
        self.publisher.publish(event);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit::HabitError;
    use crate::habit_management::domain::habit_board::HabitBoard;
    use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::infrastructure::in_memory_habit_board_repository::InMemoryHabitBoardRepository;
    use crate::habit_management::infrastructure::in_memory_outbox::InMemoryOutbox;
    use std::cell::Cell;

    struct StubGuidGenerator {
        guid: String,
    }

    impl GuidGenerator for StubGuidGenerator {
        fn generate(&self) -> String {
            self.guid.clone()
        }
    }

    /// Hands out a different guid on every call, so a use case that asked for
    /// one id and reused it would be caught.
    struct CountingGuidGenerator {
        calls: Cell<u32>,
    }

    impl GuidGenerator for CountingGuidGenerator {
        fn generate(&self) -> String {
            let next = self.calls.get() + 1;
            self.calls.set(next);
            format!("guid-{next}")
        }
    }

    /// Hands out an id longer than HabitId::MAX_LEN, so the use case's own
    /// refusal path (not the adapter's id-generation logic) is what gets
    /// exercised.
    struct OutOfBoundGuidGenerator;

    impl GuidGenerator for OutOfBoundGuidGenerator {
        fn generate(&self) -> String {
            "h".repeat(HabitId::MAX_LEN + 1)
        }
    }

    fn request_habit_with(
        guid: &str,
        publisher: Rc<dyn DomainEventPublisher>,
        board_repository: Rc<dyn HabitBoardRepository>,
    ) -> RequestHabit {
        RequestHabit::new(
            Box::new(StubGuidGenerator {
                guid: guid.to_string(),
            }),
            publisher,
            board_repository,
        )
    }

    fn a_fresh_request_habit() -> (RequestHabit, Rc<InMemoryOutbox>) {
        let outbox = Rc::new(InMemoryOutbox::new());
        let board_repository = Rc::new(InMemoryHabitBoardRepository::new());
        let request_habit = request_habit_with(
            "fixed-guid",
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
            board_repository as Rc<dyn HabitBoardRepository>,
        );
        (request_habit, outbox)
    }

    // @scenario: request-habit/S1
    #[test]
    fn requesting_a_habit_publishes_exactly_one_habit_requested_event() {
        let cases = vec![
            (String::from("Read one page"), 5),
            ("a".repeat(HabitTitle::MIN_LEN), 5),
            ("a".repeat(HabitTitle::MAX_LEN), 5),
        ];

        for (title, goal) in cases {
            let (request_habit, outbox) = a_fresh_request_habit();

            let result = request_habit.execute(title.clone(), goal);

            assert_eq!(result, Ok(HabitId::new("fixed-guid").unwrap()));
            assert_eq!(
                outbox.drain(),
                vec![HabitBoardEvent::HabitRequested {
                    id: HabitId::new("fixed-guid").unwrap(),
                    title: HabitTitle::new(title).unwrap(),
                    goal: Goal::new(goal).unwrap(),
                }]
            );
        }
    }

    #[test]
    fn every_accepted_request_gets_its_own_freshly_generated_id() {
        let outbox = Rc::new(InMemoryOutbox::new());
        let request_habit = RequestHabit::new(
            Box::new(CountingGuidGenerator {
                calls: Cell::new(0),
            }),
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
            Rc::new(InMemoryHabitBoardRepository::new()) as Rc<dyn HabitBoardRepository>,
        );

        let first = request_habit.execute(String::from("Read one page"), 5);
        let second = request_habit.execute(String::from("Move a little"), 5);

        assert_eq!(first, Ok(HabitId::new("guid-1").unwrap()));
        assert_eq!(second, Ok(HabitId::new("guid-2").unwrap()));
        let published: Vec<HabitId> = outbox
            .drain()
            .into_iter()
            .map(|event| match event {
                HabitBoardEvent::HabitRequested { id, .. } => id,
            })
            .collect();
        assert_eq!(
            published,
            vec![
                HabitId::new("guid-1").unwrap(),
                HabitId::new("guid-2").unwrap()
            ]
        );
    }

    // @scenario: request-habit/S2
    #[test]
    fn a_goal_above_the_old_five_minute_ceiling_is_accepted() {
        let (request_habit, outbox) = a_fresh_request_habit();

        let result = request_habit.execute(String::from("Run a marathon"), 6);

        assert!(result.is_ok());
        assert_eq!(outbox.drain().len(), 1);
    }

    // Moved up from a HabitError unit test on PR #1 review: only use-case and
    // service tests may pin a domain principle. Reading the reasons through
    // HabitBoardError also pins that InvalidHabit renders its inner reason
    // rather than a wrapper message of its own.
    #[test]
    fn an_invalid_habit_states_its_reason_in_the_expected_words() {
        let cases = vec![
            (
                HabitError::GoalTooSmall { min: 1 },
                "a goal must be at least 1 minute(s) per day",
            ),
            (
                HabitError::TitleLength { min: 1, max: 50 },
                "a title size must be between 1 and 50 characters",
            ),
            (
                HabitError::IdLength { min: 1, max: 64 },
                "an id size must be between 1 and 64 characters",
            ),
        ];

        for (reason, expected) in cases {
            assert_eq!(HabitBoardError::InvalidHabit(reason).to_string(), expected);
        }
    }

    // @scenario: request-habit/S3
    #[test]
    fn requesting_an_invalid_habit_returns_an_error_and_publishes_nothing() {
        let cases: Vec<(String, u32, HabitBoardError)> = vec![
            (
                String::from("Tiny"),
                0,
                HabitBoardError::InvalidHabit(HabitError::GoalTooSmall { min: 1 }),
            ),
            (
                String::new(),
                5,
                HabitBoardError::InvalidHabit(HabitError::TitleLength {
                    min: HabitTitle::MIN_LEN,
                    max: HabitTitle::MAX_LEN,
                }),
            ),
            (
                "a".repeat(HabitTitle::MAX_LEN + 1),
                5,
                HabitBoardError::InvalidHabit(HabitError::TitleLength {
                    min: HabitTitle::MIN_LEN,
                    max: HabitTitle::MAX_LEN,
                }),
            ),
        ];

        for (title, goal, expected_error) in cases {
            let (request_habit, outbox) = a_fresh_request_habit();

            let result = request_habit.execute(title, goal);

            assert_eq!(result, Err(expected_error));
            assert!(outbox.drain().is_empty());
        }
    }

    // No Gherkin scenario names this path yet either (invalid-generated-id
    // refusal, T1 conformance with adr-0001) — flagged under "Open questions",
    // matching the same gap already noted in get_habit_detail.rs / mark_done.rs.
    #[test]
    fn a_generated_id_outside_the_bound_is_refused_and_publishes_nothing() {
        let outbox = Rc::new(InMemoryOutbox::new());
        let request_habit = RequestHabit::new(
            Box::new(OutOfBoundGuidGenerator),
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
            Rc::new(InMemoryHabitBoardRepository::new()) as Rc<dyn HabitBoardRepository>,
        );

        let result = request_habit.execute(String::from("Read one page"), 5);

        assert_eq!(
            result,
            Err(HabitBoardError::InvalidHabit(HabitError::IdLength {
                min: HabitId::MIN_LEN,
                max: HabitId::MAX_LEN,
            }))
        );
        assert!(outbox.drain().is_empty());
    }

    fn a_full_board() -> (Rc<InMemoryOutbox>, Rc<InMemoryHabitBoardRepository>) {
        let outbox = Rc::new(InMemoryOutbox::new());
        let board_repository = Rc::new(InMemoryHabitBoardRepository::new());

        for n in 1..=HabitBoard::MAX_HABITS {
            let request_habit = request_habit_with(
                &format!("guid-{n}"),
                Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
                Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
            );

            let result = request_habit.execute(format!("Habit number {n}"), 1);

            assert!(result.is_ok());
        }

        (outbox, board_repository)
    }

    // @scenario: request-habit/S4
    #[test]
    fn a_sixth_habit_request_on_a_full_board_is_rejected_and_publishes_nothing() {
        assert_eq!(HabitBoard::MAX_HABITS, 5);

        let (outbox, board_repository) = a_full_board();

        let sixth_request_habit = request_habit_with(
            "guid-6",
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
            Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
        );

        let result = sixth_request_habit.execute(String::from("One habit too many"), 1);

        assert_eq!(
            result,
            Err(HabitBoardError::BoardFull {
                max: HabitBoard::MAX_HABITS
            })
        );
        assert_eq!(outbox.drain().len(), HabitBoard::MAX_HABITS);
    }

    // @scenario: request-habit/S5
    #[test]
    fn requesting_a_duplicate_title_is_rejected_and_publishes_nothing() {
        let outbox = Rc::new(InMemoryOutbox::new());
        let board_repository = Rc::new(InMemoryHabitBoardRepository::new());

        let first = request_habit_with(
            "guid-1",
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
            Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
        );
        let second = request_habit_with(
            "guid-2",
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
            Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
        );

        let first_result = first.execute(String::from("Lire une page"), 2);
        let second_result = second.execute(String::from("lire une page "), 2);

        assert!(first_result.is_ok());
        assert_eq!(second_result, Err(HabitBoardError::DuplicateHabit));
        assert_eq!(outbox.drain().len(), 1);
    }

    // @scenario: request-habit/S6
    #[test]
    fn a_duplicate_title_on_a_full_board_is_rejected_as_duplicate_not_full() {
        let (outbox, board_repository) = a_full_board();

        let duplicate_request_habit = request_habit_with(
            "guid-6",
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
            Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
        );

        let result = duplicate_request_habit.execute(String::from("Habit number 1"), 1);

        assert_eq!(result, Err(HabitBoardError::DuplicateHabit));
        assert_eq!(outbox.drain().len(), HabitBoard::MAX_HABITS);
    }
}
