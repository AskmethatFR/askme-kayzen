use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;

#[derive(Debug, PartialEq)]
pub enum PauseHabitError {
    HabitNotFound,
}

impl fmt::Display for PauseHabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PauseHabitError::HabitNotFound => write!(f, "no habit with this id is on the board"),
        }
    }
}

impl Error for PauseHabitError {}

/// Command use case: pauses a habit, sans culpabilité — no `Clock` (adr-0007
/// AD-3): nothing about this transition is dated.
#[derive(Clone)]
pub struct PauseHabit {
    repository: Rc<dyn HabitRepository>,
}

impl PauseHabit {
    pub fn new(repository: Rc<dyn HabitRepository>) -> PauseHabit {
        PauseHabit { repository }
    }

    pub fn execute(&self, habit_id: &str) -> Result<(), PauseHabitError> {
        let id = HabitId::new(habit_id).map_err(|_| PauseHabitError::HabitNotFound)?;
        let mut habit = self
            .repository
            .get(&id)
            .ok_or(PauseHabitError::HabitNotFound)?;
        habit.pause();
        self.repository.save(&habit);
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
    use crate::habit_management::domain::habit_board_repository::HabitBoardRepository;
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

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            PauseHabitError::HabitNotFound.to_string(),
            "no habit with this id is on the board"
        );
    }

    fn a_habit(id: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn pause_habit_over(repository: Rc<InMemoryHabitRepository>) -> PauseHabit {
        PauseHabit::new(repository as Rc<dyn HabitRepository>)
    }

    // @scenario: pause-resume/S1
    #[test]
    fn pausing_an_active_habit_leaves_it_paused_in_the_store() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let pause_habit = pause_habit_over(Rc::clone(&repository));

        let result = pause_habit.execute("h-1");

        assert_eq!(result, Ok(()));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.state(), LifecycleState::Paused);
    }

    #[test]
    fn pausing_an_unknown_habit_is_rejected() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let pause_habit = pause_habit_over(repository);

        let result = pause_habit.execute("missing");

        assert_eq!(result, Err(PauseHabitError::HabitNotFound));
    }

    #[test]
    fn pausing_an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1"));
        let pause_habit = pause_habit_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        let result = pause_habit.execute(&too_long);

        assert_eq!(result, Err(PauseHabitError::HabitNotFound));
    }

    struct StubGuidGenerator {
        guid: String,
    }

    impl GuidGenerator for StubGuidGenerator {
        fn generate(&self) -> String {
            self.guid.clone()
        }
    }

    // @scenario: pause-resume/S3
    #[test]
    fn a_paused_habit_keeps_its_seat_so_a_sixth_request_is_still_rejected() {
        assert_eq!(HabitBoard::MAX_HABITS, 5);

        let outbox = Rc::new(InMemoryOutbox::new());
        let board_repository = Rc::new(InMemoryHabitBoardRepository::new());
        let habit_repository = Rc::new(InMemoryHabitRepository::new());
        let clock: Rc<dyn Clock> = Rc::new(FixedClock::new(LocalDate::from_epoch_day(CREATED_ON)));
        let create_habit_on_request =
            CreateHabitOnRequest::new(Rc::clone(&habit_repository) as Rc<dyn HabitRepository>, clock);

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

        let pause_habit = pause_habit_over(Rc::clone(&habit_repository));
        pause_habit.execute("guid-1").expect("known habit");

        let sixth_request_habit = RequestHabit::new(
            Box::new(StubGuidGenerator {
                guid: "guid-6".to_string(),
            }),
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
        let paused = habit_repository
            .get(&HabitId::new("guid-1").unwrap())
            .unwrap();
        assert_eq!(
            paused.state(),
            LifecycleState::Paused,
            "the paused habit itself stays paused across the rejected sixth request"
        );
    }
}
