use std::rc::Rc;

use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
use kayzen_core::habit_management::queries::get_habit_detail::GetHabitDetail;
use kayzen_core::habit_management::queries::get_week_recap::GetWeekRecap;
use kayzen_core::habit_management::queries::list_anchored_habits::ListAnchoredHabits;
use kayzen_core::habit_management::queries::list_board_habits::ListBoardHabits;
use kayzen_core::habit_management::use_cases::add_habit::AddHabit;
use kayzen_core::habit_management::use_cases::anchor_habit::AnchorHabit;
use kayzen_core::habit_management::use_cases::grow_goal::GrowGoal;
use kayzen_core::habit_management::use_cases::lighten_goal::LightenGoal;
use kayzen_core::habit_management::use_cases::mark_done::MarkDone;
use kayzen_core::habit_management::use_cases::pause_habit::PauseHabit;
use kayzen_core::habit_management::use_cases::readmit_habit::ReadmitHabit;
use kayzen_core::habit_management::use_cases::resume_habit::ResumeHabit;
use kayzen_core::shared::clock::{Clock, SystemClock};
use kayzen_core::shared::guid_generator::UuidGenerator;

/// The default daily goal offered to every new habit — a flexible target, not
/// a ceiling. Kaizen begins gently, not necessarily tiny.
pub(crate) const STARTING_GOAL: u32 = 5;

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
    pub get_week_recap: GetWeekRecap,
    pub grow_goal: GrowGoal,
    pub lighten_goal: LightenGoal,
    pub pause_habit: PauseHabit,
    pub resume_habit: ResumeHabit,
    pub readmit_habit: ReadmitHabit,
    pub anchor_habit: AnchorHabit,
    pub list_anchored_habits: ListAnchoredHabits,
}

impl Services {
    pub fn new() -> Self {
        Self::with_repository(Rc::new(InMemoryHabitRepository::new()))
    }

    /// Wires every service over a caller-provided habit store, resolving
    /// "today" from the real system clock. Testability seam: tests inject a
    /// habit store seeded with known data and assert what the screens render.
    pub fn with_repository(habit_repository: Rc<dyn HabitRepository>) -> Self {
        Self::with_repository_and_clock(habit_repository, Rc::new(SystemClock))
    }

    /// Same wiring as `with_repository`, but with the clock also injected. This
    /// is the seam a test needs when it stamps a habit as done "today" itself:
    /// injecting the same clock on both sides makes the two reads agree by
    /// construction instead of by luck between two independent `SystemClock`
    /// reads.
    pub fn with_repository_and_clock(
        habit_repository: Rc<dyn HabitRepository>,
        clock: Rc<dyn Clock>,
    ) -> Self {
        Services {
            list_board_habits: ListBoardHabits::new(
                Rc::clone(&habit_repository),
                Rc::clone(&clock),
            ),
            mark_done: MarkDone::new(Rc::clone(&habit_repository), Rc::clone(&clock)),
            get_habit_detail: GetHabitDetail::new(Rc::clone(&habit_repository), Rc::clone(&clock)),
            get_week_recap: GetWeekRecap::new(Rc::clone(&habit_repository), Rc::clone(&clock)),
            grow_goal: GrowGoal::new(Rc::clone(&habit_repository), Rc::clone(&clock)),
            lighten_goal: LightenGoal::new(Rc::clone(&habit_repository), Rc::clone(&clock)),
            pause_habit: PauseHabit::new(Rc::clone(&habit_repository)),
            resume_habit: ResumeHabit::new(Rc::clone(&habit_repository)),
            readmit_habit: ReadmitHabit::new(Rc::clone(&habit_repository)),
            anchor_habit: AnchorHabit::new(Rc::clone(&habit_repository)),
            list_anchored_habits: ListAnchoredHabits::new(Rc::clone(&habit_repository)),
            add_habit: AddHabit::new(habit_repository, Rc::new(UuidGenerator), clock),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Services;

    #[test]
    fn new_creates_no_habit() {
        let services = Services::new();

        let board = services.list_board_habits.handle();

        assert!(board.is_empty());
    }
}
