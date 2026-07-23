use std::cell::RefCell;

use crate::habit_management::domain::domain_event_publisher::DomainEventPublisher;
use crate::habit_management::domain::habit_board_event::HabitBoardEvent;

#[derive(Default)]
pub struct InMemoryOutbox {
    events: RefCell<Vec<HabitBoardEvent>>,
}

impl InMemoryOutbox {
    pub fn new() -> InMemoryOutbox {
        InMemoryOutbox::default()
    }

    pub fn drain(&self) -> Vec<HabitBoardEvent> {
        std::mem::take(&mut self.events.borrow_mut())
    }
}

impl DomainEventPublisher for InMemoryOutbox {
    fn publish(&self, event: HabitBoardEvent) {
        self.events.borrow_mut().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::habit_title::HabitTitle;

    fn a_request(id: &str) -> HabitBoardEvent {
        HabitBoardEvent::HabitRequested {
            id: HabitId::from(id),
            title: HabitTitle::new(String::from("Read one page")).unwrap(),
            goal: Goal::new(2).unwrap(),
        }
    }

    #[test]
    fn publishes_events_then_drains_them_in_order() {
        let outbox = InMemoryOutbox::new();

        outbox.publish(a_request("id-1"));
        outbox.publish(a_request("id-2"));

        assert_eq!(outbox.drain(), vec![a_request("id-1"), a_request("id-2")]);
    }

    #[test]
    fn draining_empties_the_outbox() {
        let outbox = InMemoryOutbox::new();
        outbox.publish(a_request("id-1"));

        outbox.drain();

        assert!(outbox.drain().is_empty());
    }
}
