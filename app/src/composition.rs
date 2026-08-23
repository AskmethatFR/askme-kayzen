use std::rc::Rc;

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
use kayzen_core::habit_management::infrastructure::persistent_habit_repository::PersistentHabitRepository;
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
    /// The real entry point: wires every service over the platform's durable
    /// store (a file on desktop, `localStorage` on web — selected at compile
    /// time, never at runtime) decorating `InMemoryHabitRepository`, so a
    /// habit added and a completion recorded both survive closing and
    /// reopening the app. No seed: whatever the store holds is the whole
    /// board, including nothing.
    pub fn new() -> Self {
        Self::with_repository(Rc::new(platform_habit_repository()))
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

#[cfg(not(target_arch = "wasm32"))]
fn platform_habit_repository() -> PersistentHabitRepository {
    persistent_habit_repository_at(&default_data_dir())
}

#[cfg(not(target_arch = "wasm32"))]
fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("kayzen")
}

/// The exact wiring `platform_habit_repository()` performs, with the
/// directory injected — the seam that lets a test prove the `cfg` selection
/// persists across a relaunch without ever touching the real user directory
/// `default_data_dir()` resolves to.
#[cfg(not(target_arch = "wasm32"))]
fn persistent_habit_repository_at(dir: &Path) -> PersistentHabitRepository {
    use crate::infrastructure::file_snapshot_store::FileSnapshotStore;

    PersistentHabitRepository::hydrated_from(
        Rc::new(FileSnapshotStore::at(dir.join("habits.json"))),
        Rc::new(FileSnapshotStore::at(dir.join("habits.unreadable.json"))),
    )
}

#[cfg(target_arch = "wasm32")]
fn platform_habit_repository() -> PersistentHabitRepository {
    use crate::infrastructure::local_storage_snapshot_store::LocalStorageSnapshotStore;

    PersistentHabitRepository::hydrated_from(
        Rc::new(LocalStorageSnapshotStore::at("kayzen.habits.v1")),
        Rc::new(LocalStorageSnapshotStore::at("kayzen.habits.unreadable")),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use kayzen_core::shared::clock::Clock;
    use kayzen_core::shared::local_date::LocalDate;

    use super::*;

    struct FixedClock(LocalDate);

    impl Clock for FixedClock {
        fn today(&self) -> LocalDate {
            self.0
        }
    }

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A fresh, never-created directory under the OS temp directory — every
    /// test gets its own, so none can observe another's data. Deliberately
    /// never touches `default_data_dir()`: that is the real user's machine,
    /// and this suite must never write there.
    fn unused_temp_dir() -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "kayzen-composition-test-{}-{}",
            std::process::id(),
            unique
        ))
    }

    #[test]
    fn with_repository_over_an_in_memory_store_creates_no_habit() {
        let services = Services::with_repository(Rc::new(InMemoryHabitRepository::new()));

        let board = services.list_board_habits.handle();

        assert!(board.is_empty());
    }

    // @scenario: persistence/S1
    #[test]
    fn a_habit_added_through_add_habit_is_listed_after_a_relaunch_over_the_same_directory() {
        let dir = unused_temp_dir();
        let first_launch = Services::with_repository(Rc::new(persistent_habit_repository_at(&dir)));
        first_launch
            .add_habit
            .execute("Lire une page".to_string(), STARTING_GOAL)
            .expect("a fresh directory has room for one habit");

        let relaunch = Services::with_repository(Rc::new(persistent_habit_repository_at(&dir)));

        let board = relaunch.list_board_habits.handle();
        assert!(
            board
                .active
                .iter()
                .any(|habit| habit.title == "Lire une page"),
            "expected the added habit to survive the relaunch, got: {board:?}"
        );
    }

    // @scenario: persistence/S2
    #[test]
    fn a_completion_recorded_through_mark_done_is_still_marked_after_a_relaunch_over_the_same_directory()
     {
        let dir = unused_temp_dir();
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_000)));
        let first_launch = Services::with_repository_and_clock(
            Rc::new(persistent_habit_repository_at(&dir)),
            Rc::clone(&clock),
        );
        first_launch
            .add_habit
            .execute("Bouger un peu".to_string(), STARTING_GOAL)
            .expect("a fresh directory has room for one habit");
        let habit_id = first_launch.list_board_habits.handle().active[0].id.clone();
        first_launch
            .mark_done
            .execute(&habit_id)
            .expect("the habit was just added");

        let relaunch = Services::with_repository_and_clock(
            Rc::new(persistent_habit_repository_at(&dir)),
            clock,
        );

        let detail = relaunch
            .get_habit_detail
            .handle(&habit_id)
            .expect("the habit survived the relaunch");
        assert!(
            detail.done_today,
            "expected today's completion to survive the relaunch"
        );
    }

    // @scenario: persistence/S4
    #[test]
    fn an_unreadable_primary_file_is_quarantined_at_the_fixed_sibling_path() {
        let dir = unused_temp_dir();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        fs::write(dir.join("habits.json"), "not-json-at-all").expect("temp file must be writable");

        let repository = persistent_habit_repository_at(&dir);

        assert_eq!(repository.all(), Vec::new());
        assert_eq!(
            fs::read_to_string(dir.join("habits.unreadable.json")).ok(),
            Some("not-json-at-all".to_string()),
            "expected the unreadable payload copied aside at the fixed quarantine path"
        );
    }

    #[test]
    fn default_data_dir_is_named_kayzen() {
        assert_eq!(
            default_data_dir().file_name(),
            Some(std::ffi::OsStr::new("kayzen"))
        );
    }
}
