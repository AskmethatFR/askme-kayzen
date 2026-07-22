use std::rc::Rc;

use kayzen_core::habit_management::domain::habit::Habit;
use kayzen_core::habit_management::domain::habit_id::HabitId;
use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
use kayzen_core::habit_management::domain::habit_title::HabitTitle;
use kayzen_core::habit_management::domain::initial_duration::InitialDuration;
use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
use kayzen_core::habit_management::queries::list_board_habits::list_board_habits::ListBoardHabits;
use kayzen_core::habit_management::use_cases::mark_done::mark_done::MarkDone;
use kayzen_core::shared::clock::{Clock, SystemClock};

/// Composition root: builds and wires every use case over a single shared store,
/// then is provided once at the app root via Dioxus context so any screen can
/// reach its use cases without prop drilling.
///
/// This is the one place that knows how the app is assembled. Future use cases
/// (request-habit, mark-done, ...) join here, constructed from the SAME repository.
#[derive(Clone)]
pub struct Services {
    pub list_board_habits: ListBoardHabits,
    pub mark_done: MarkDone,
}

impl Services {
    pub fn new() -> Self {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());

        // TODO: remove once the AddHabit screen is wired to the request-habit
        // use case (slice mark-done / request flow). Until then, seed a few
        // habits so the Today screen renders real data end-to-end.
        seed_demo_habits(&repository);

        Self::with_repository(repository)
    }

    /// Wires the use cases over a caller-provided store. Testability seam: tests
    /// inject a repository seeded with known data and assert what the screens render.
    pub fn with_repository(repository: Rc<dyn HabitRepository>) -> Self {
        let clock: Rc<dyn Clock> = Rc::new(SystemClock);

        Services {
            list_board_habits: ListBoardHabits::new(Rc::clone(&repository), Rc::clone(&clock)),
            mark_done: MarkDone::new(repository, clock),
        }
    }
}

fn seed_demo_habits(repository: &Rc<dyn HabitRepository>) {
    let demo = [("Lire une page", 2u32), ("Bouger un peu", 3), ("Respirer", 1)];

    for (index, (title, minutes)) in demo.iter().enumerate() {
        let habit = Habit::new(
            HabitId::new(format!("demo-{index}")),
            HabitTitle::new(title.to_string()).expect("valid demo title"),
            InitialDuration::new(*minutes).expect("valid demo duration"),
        );
        repository.save(&habit);
    }
}
