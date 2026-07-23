pub mod domain {
    pub mod completion_history;
    pub mod domain_event_publisher;
    pub mod goal;
    pub mod habit;
    pub mod habit_board;
    pub mod habit_board_event;
    pub mod habit_board_repository;
    pub mod habit_id;
    pub mod habit_repository;
    pub mod habit_title;
}

pub mod infrastructure {
    pub mod in_memory_habit_board_repository;
    pub mod in_memory_habit_repository;
    pub mod in_memory_outbox;
}

pub mod use_cases {
    pub mod create_habit_on_request;
    pub mod mark_done;
    pub mod request_habit;
}

pub mod queries {
    pub mod list_board_habits;
}
