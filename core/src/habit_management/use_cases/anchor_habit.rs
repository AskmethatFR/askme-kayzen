use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::habit_management::domain::habit_board_repository::HabitBoardRepository;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;

#[derive(Debug, PartialEq)]
pub enum AnchorHabitError {
    HabitNotFound,
}

impl fmt::Display for AnchorHabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AnchorHabitError::HabitNotFound => write!(f, "no habit with this id is on the board"),
        }
    }
}

impl Error for AnchorHabitError {}

/// Command use case: anchors a habit that has become natural and frees its
/// seat on the board. No `Clock` (adr-0007 AD-3): nothing about this
/// transition is dated. Coordinates both aggregates synchronously and
/// publishes nothing (adr-0007 d3); both steps are idempotent, so there is
/// no transaction to reach for — replaying the gesture converges.
#[derive(Clone)]
pub struct AnchorHabit {
    repository: Rc<dyn HabitRepository>,
    board_repository: Rc<dyn HabitBoardRepository>,
}

impl AnchorHabit {
    pub fn new(
        repository: Rc<dyn HabitRepository>,
        board_repository: Rc<dyn HabitBoardRepository>,
    ) -> AnchorHabit {
        AnchorHabit {
            repository,
            board_repository,
        }
    }

    pub fn execute(&self, habit_id: &str) -> Result<(), AnchorHabitError> {
        let id = HabitId::new(habit_id).map_err(|_| AnchorHabitError::HabitNotFound)?;
        let mut habit = self
            .repository
            .get(&id)
            .ok_or(AnchorHabitError::HabitNotFound)?;
        habit.anchor();
        self.repository.save(&habit);

        let mut board = self.board_repository.load();
        board.release(&id);
        self.board_repository.save(&board);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::domain_event_publisher::DomainEventPublisher;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_board::HabitBoard;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::domain::lifecycle_state::LifecycleState;
    use crate::habit_management::infrastructure::in_memory_habit_board_repository::InMemoryHabitBoardRepository;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::habit_management::infrastructure::in_memory_outbox::InMemoryOutbox;
    use crate::habit_management::use_cases::create_habit_on_request::CreateHabitOnRequest;
    use crate::habit_management::use_cases::request_habit::RequestHabit;
    use crate::shared::clock::{Clock, FixedClock};
    use crate::shared::guid_generator::GuidGenerator;
    use crate::shared::local_date::LocalDate;

    const CREATED_ON: i64 = 19_990;

    fn a_habit(id: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn anchor_habit_over(repository: Rc<InMemoryHabitRepository>) -> AnchorHabit {
        AnchorHabit::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(InMemoryHabitBoardRepository::new()) as Rc<dyn HabitBoardRepository>,
        )
    }

    struct StubGuidGenerator {
        guid: String,
    }

    impl GuidGenerator for StubGuidGenerator {
        fn generate(&self) -> String {
            self.guid.clone()
        }
    }

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            AnchorHabitError::HabitNotFound.to_string(),
            "no habit with this id is on the board"
        );
    }

    // @scenario: anchor-habit/S2
    #[test]
    fn anchoring_an_active_habit_marks_it_anchored_in_the_store() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let anchor_habit = anchor_habit_over(Rc::clone(&repository));

        let result = anchor_habit.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.state(), LifecycleState::Anchored);
    }

    #[test]
    fn anchoring_an_unknown_habit_is_rejected() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let anchor_habit = anchor_habit_over(repository);

        let result = anchor_habit.execute("missing");

        assert_eq!(result, Err(AnchorHabitError::HabitNotFound));
    }

    #[test]
    fn anchoring_an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let anchor_habit = anchor_habit_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        let result = anchor_habit.execute(&too_long);

        assert_eq!(result, Err(AnchorHabitError::HabitNotFound));
    }

    // @scenario: anchor-habit/S1
    #[test]
    fn anchoring_a_habit_frees_its_seat_so_a_sixth_request_is_now_accepted() {
        assert_eq!(HabitBoard::MAX_HABITS, 5);

        let outbox = Rc::new(InMemoryOutbox::new());
        let board_repository = Rc::new(InMemoryHabitBoardRepository::new());
        let habit_repository = Rc::new(InMemoryHabitRepository::new());
        let clock: Rc<dyn Clock> = Rc::new(FixedClock::new(LocalDate::from_epoch_day(CREATED_ON)));
        let create_habit_on_request = CreateHabitOnRequest::new(
            Rc::clone(&habit_repository) as Rc<dyn HabitRepository>,
            clock,
        );

        for n in 1..=HabitBoard::MAX_HABITS {
            let request_habit = RequestHabit::new(
                Box::new(StubGuidGenerator {
                    guid: format!("guid-{n}"),
                }),
                Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
                Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
            );
            request_habit
                .execute(format!("Habit number {n}"), 1)
                .expect("valid habit request");
        }
        for event in outbox.drain() {
            create_habit_on_request.handle(event);
        }

        let anchor_habit = AnchorHabit::new(
            Rc::clone(&habit_repository) as Rc<dyn HabitRepository>,
            Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
        );
        anchor_habit.execute("guid-1").expect("known habit");

        let sixth_request_habit = RequestHabit::new(
            Box::new(StubGuidGenerator {
                guid: "guid-6".to_string(),
            }),
            Rc::clone(&outbox) as Rc<dyn DomainEventPublisher>,
            Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
        );

        let result = sixth_request_habit.execute(String::from("One habit too many"), 1);

        assert!(
            result.is_ok(),
            "expected the anchored habit's seat to have been freed, got: {result:?}"
        );
        let anchored = habit_repository
            .get(&HabitId::new("guid-1").unwrap())
            .unwrap();
        assert_eq!(
            anchored.state(),
            LifecycleState::Anchored,
            "the anchored habit itself stays anchored — anchoring frees the seat, not the habit"
        );
    }
}
