pub mod domain {
    pub mod completion_history;
    pub mod goal;
    pub mod habit;
    pub mod habit_id;
    pub mod habit_repository;
    pub mod habit_title;
    pub mod lifecycle_state;
    pub mod step_history;
}

pub mod infrastructure {
    pub mod habit_snapshot_codec;
    pub mod in_memory_habit_repository;
    pub mod persistent_habit_repository;
    pub mod snapshot_store;
}

pub mod use_cases {
    pub mod add_habit;
    pub mod anchor_habit;
    pub mod grow_goal;
    pub mod lighten_goal;
    pub mod mark_done;
    pub mod pause_habit;
    pub mod readmit_habit;
    pub mod resume_habit;
}

pub mod queries {
    pub mod get_habit_detail;
    pub mod get_week_recap;
    pub mod list_anchored_habits;
    pub mod list_board_habits;
}
