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
    use crate::habit_management::domain::habit_board::{HabitBoard, HabitBoardError};
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

    fn a_request_habit(
        guid: &str,
        board_repository: &Rc<InMemoryHabitBoardRepository>,
        outbox: &Rc<InMemoryOutbox>,
    ) -> RequestHabit {
        RequestHabit::new(
            Box::new(StubGuidGenerator {
                guid: guid.to_string(),
            }),
            Rc::clone(outbox) as Rc<dyn DomainEventPublisher>,
            Rc::clone(board_repository) as Rc<dyn HabitBoardRepository>,
        )
    }

    /// Requests one habit per title (guid-1, guid-2, ...) and drains the
    /// outbox through CreateHabitOnRequest, so every caller starts from a
    /// board and a habit store that already agree — the exact wiring S1/C3
    /// both need before they can anchor anything.
    fn a_board_seeded_with(
        titles: &[&str],
    ) -> (
        Rc<InMemoryHabitRepository>,
        Rc<InMemoryHabitBoardRepository>,
        Rc<InMemoryOutbox>,
    ) {
        let outbox = Rc::new(InMemoryOutbox::new());
        let board_repository = Rc::new(InMemoryHabitBoardRepository::new());
        let habit_repository = Rc::new(InMemoryHabitRepository::new());
        let clock: Rc<dyn Clock> = Rc::new(FixedClock::new(LocalDate::from_epoch_day(CREATED_ON)));
        let create_habit_on_request = CreateHabitOnRequest::new(
            Rc::clone(&habit_repository) as Rc<dyn HabitRepository>,
            clock,
        );

        for (n, title) in titles.iter().enumerate() {
            let guid = format!("guid-{}", n + 1);
            a_request_habit(&guid, &board_repository, &outbox)
                .execute(title.to_string(), 1)
                .expect("valid habit request");
        }
        for event in outbox.drain() {
            create_habit_on_request.handle(event);
        }

        (habit_repository, board_repository, outbox)
    }

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            AnchorHabitError::HabitNotFound.to_string(),
            "no habit with this id is on the board"
        );
    }

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

        let titles = [
            "Habit number 1",
            "Habit number 2",
            "Habit number 3",
            "Habit number 4",
            "Habit number 5",
        ];
        let (habit_repository, board_repository, outbox) = a_board_seeded_with(&titles);

        let anchor_habit = AnchorHabit::new(
            Rc::clone(&habit_repository) as Rc<dyn HabitRepository>,
            Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
        );
        anchor_habit.execute("guid-1").expect("known habit");

        let result = a_request_habit("guid-6", &board_repository, &outbox)
            .execute(String::from("One habit too many"), 1);

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

    // C3: freeing the seat also frees the title — release drops exactly the
    // anchored entry, not every other one.
    #[test]
    fn anchoring_a_habit_frees_its_title_while_the_others_stay_taken() {
        let (habit_repository, board_repository, outbox) =
            a_board_seeded_with(&["Read one page", "Move a little"]);

        let anchor_habit = AnchorHabit::new(
            Rc::clone(&habit_repository) as Rc<dyn HabitRepository>,
            Rc::clone(&board_repository) as Rc<dyn HabitBoardRepository>,
        );
        anchor_habit.execute("guid-1").expect("known habit");

        assert!(
            a_request_habit("guid-3", &board_repository, &outbox)
                .execute("Read one page".to_string(), 1)
                .is_ok(),
            "expected the anchored habit's title to have been freed"
        );

        assert_eq!(
            a_request_habit("guid-4", &board_repository, &outbox)
                .execute("Move a little".to_string(), 1),
            Err(HabitBoardError::DuplicateHabit),
            "expected the still-active habit's title to stay taken — release must \
             drop exactly the anchored entry, not every other one"
        );
    }
}
