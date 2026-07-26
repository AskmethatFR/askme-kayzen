use std::rc::Rc;

use kayzen_core::habit_management::domain::habit_board_repository::HabitBoardRepository;
use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
use kayzen_core::habit_management::infrastructure::in_memory_habit_board_repository::InMemoryHabitBoardRepository;
use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
use kayzen_core::habit_management::infrastructure::in_memory_outbox::InMemoryOutbox;
use kayzen_core::habit_management::queries::get_habit_detail::GetHabitDetail;
use kayzen_core::habit_management::queries::list_board_habits::ListBoardHabits;
use kayzen_core::habit_management::use_cases::mark_done::MarkDone;
use kayzen_core::shared::clock::{Clock, SystemClock};

use crate::services::add_habit::AddHabit;

/// Composition root: a pure DI registry. It builds and holds each action service
/// over a single shared set of stores, then is provided once at the app root via
/// Dioxus context so any screen reaches its services without prop drilling. It
/// carries no business logic — every action lives in its own type, added here as
/// one more field per slice.
#[derive(Clone)]
pub struct Services {
    pub list_board_habits: ListBoardHabits,
    pub mark_done: MarkDone,
    pub add_habit: AddHabit,
    pub get_habit_detail: GetHabitDetail,
}

impl Services {
    pub fn new() -> Self {
        let services = Self::with_repository(Rc::new(InMemoryHabitRepository::new()));

        // TODO: remove this demo seed once the AddHabit screen lets the user
        // create habits themselves.
        for title in ["Lire une page", "Bouger un peu", "Respirer"] {
            let _ = services.add_habit.execute(title);
        }

        services
    }

    /// Wires every service over a caller-provided habit store (the board store and
    /// outbox are created fresh alongside it). Testability seam: tests inject a
    /// habit store seeded with known data and assert what the screens render.
    pub fn with_repository(habit_repository: Rc<dyn HabitRepository>) -> Self {
        let board_repository: Rc<dyn HabitBoardRepository> =
            Rc::new(InMemoryHabitBoardRepository::new());
        let outbox = Rc::new(InMemoryOutbox::new());
        let clock: Rc<dyn Clock> = Rc::new(SystemClock);

        Services {
            list_board_habits: ListBoardHabits::new(
                Rc::clone(&habit_repository),
                Rc::clone(&clock),
            ),
            mark_done: MarkDone::new(Rc::clone(&habit_repository), Rc::clone(&clock)),
            get_habit_detail: GetHabitDetail::new(Rc::clone(&habit_repository)),
            add_habit: AddHabit::new(
                Rc::clone(&habit_repository),
                board_repository,
                outbox,
                clock,
            ),
        }
    }
}
