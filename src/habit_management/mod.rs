pub mod domain {
    pub mod domain_event_publisher;
    pub mod habit;
    pub mod habit_board;
    pub mod habit_board_event;
    pub mod habit_id;
    pub mod habit_repository;
}

pub mod infrastructure {
    pub mod in_memory_habit_repository;
    pub mod in_memory_outbox;
}

#[path = "use-cases"]
pub mod use_cases {
    #[path = "create-habit"]
    pub mod create_habit {
        #[path = "create-habit-command.rs"]
        pub mod create_habit_command;
    }
}
