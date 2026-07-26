use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_board_event::HabitBoardEvent;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::shared::clock::Clock;
use std::rc::Rc;

pub struct CreateHabitOnRequest {
    repository: Rc<dyn HabitRepository>,
    clock: Rc<dyn Clock>,
}

impl CreateHabitOnRequest {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> CreateHabitOnRequest {
        CreateHabitOnRequest { repository, clock }
    }

    pub fn handle(&self, event: HabitBoardEvent) {
        match event {
            HabitBoardEvent::HabitRequested { id, title, goal } => {
                let habit = Habit::new(id, title, goal, self.clock.today());
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
    use crate::shared::clock::FixedClock;
    use crate::shared::guid_generator::GuidGenerator;
    use crate::shared::local_date::LocalDate;

    const CREATED_ON: i64 = 20_000;

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
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn a_habit_requested(id: &str, title: &str, goal: u32) -> HabitBoardEvent {
        HabitBoardEvent::HabitRequested {
            id: HabitId::from(id),
            title: HabitTitle::new(String::from(title)).unwrap(),
            goal: Goal::new(goal).unwrap(),
        }
    }

    fn create_habit_on_request_over(
        repository: Rc<InMemoryHabitRepository>,
    ) -> CreateHabitOnRequest {
        CreateHabitOnRequest::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(CREATED_ON))) as Rc<dyn Clock>,
        )
    }

    // @scenario: create-habit-on-request/S1
    #[test]
    fn handling_a_habit_requested_event_persists_the_habit() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let handler = create_habit_on_request_over(Rc::clone(&repository));

        handler.handle(a_habit_requested("id-1", "Read one page", 2));

        assert_eq!(repository.all(), vec![a_habit("id-1", "Read one page", 2)]);
    }

    // Test List — CreateHabitOnRequest threading the Clock (adr-0007 AD-1):
    // - the persisted habit's first step is dated with the clock's "today",
    //   not a default/hardcoded date.
    #[test]
    fn the_persisted_habit_is_created_on_the_clocks_today() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let handler = create_habit_on_request_over(Rc::clone(&repository));

        handler.handle(a_habit_requested("id-1", "Read one page", 2));

        let habit = repository.get(&HabitId::from("id-1")).unwrap();
        let dates: Vec<LocalDate> = habit
            .step_history()
            .changes()
            .into_iter()
            .map(|step| step.on())
            .collect();
        assert_eq!(dates, vec![LocalDate::from_epoch_day(CREATED_ON)]);
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
        let handler = create_habit_on_request_over(Rc::clone(&repository));

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
