use super::habit_board_event::HabitBoardEvent;

pub trait DomainEventPublisher {
    fn publish(&self, event: HabitBoardEvent);
}
