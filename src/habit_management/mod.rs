pub mod domain {
    pub mod domain_event_publisher;
    pub mod habit;
    pub mod habit_board;
    pub mod habit_board_event;
    pub mod habit_board_repository;
    pub mod habit_id;
    pub mod habit_repository;
    pub mod habit_title;
    pub mod initial_duration;
}

pub mod infrastructure {
    pub mod in_memory_habit_board_repository;
    pub mod in_memory_habit_repository;
    pub mod in_memory_outbox;
}

#[path = "use-cases"]
pub mod use_cases {
    #[path = "request-habit"]
    pub mod request_habit {
        #[path = "request-habit.rs"]
        pub mod request_habit;
    }

    #[path = "create-habit-on-request"]
    pub mod create_habit_on_request {
        #[path = "create-habit-on-request.rs"]
        pub mod create_habit_on_request;
    }
}
