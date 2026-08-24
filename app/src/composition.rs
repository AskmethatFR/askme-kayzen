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
    /// board, including nothing. `None` when the platform offers no durable
    /// place to keep habits at all (desktop/mobile only — neither
    /// `dirs::data_dir()` nor, on Android, `Context.getFilesDir()` over JNI
    /// resolved a usable directory) — the caller renders a refusal screen
    /// instead of pretending
    /// to save.
    pub fn new() -> Option<Self> {
        Some(Self::with_repository(Rc::new(platform_habit_repository()?)))
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
fn platform_habit_repository() -> Option<PersistentHabitRepository> {
    platform_habit_repository_from(default_data_dir())
}

/// The exact wiring `platform_habit_repository()` performs, with the
/// resolved data directory injected — the seam a test needs to prove that
/// `None` (no durable place to store habits) never reaches
/// `persistent_habit_repository_at`, the only place a `FileSnapshotStore`
/// gets built and could touch disk.
#[cfg(not(target_arch = "wasm32"))]
fn platform_habit_repository_from(data_dir: Option<PathBuf>) -> Option<PersistentHabitRepository> {
    data_dir.map(|dir| persistent_habit_repository_at(&dir))
}

/// Android has no `HOME`/`XDG_DATA_HOME`, so `dirs::data_dir()` (which
/// routes Android through its Linux/XDG lookup) always returns `None`
/// there. `Context.getFilesDir()` over JNI is Android's own answer to the
/// same question.
#[cfg(target_os = "android")]
fn default_data_dir() -> Option<PathBuf> {
    resolve_data_dir(crate::infrastructure::android_files_dir::files_dir())
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn default_data_dir() -> Option<PathBuf> {
    resolve_data_dir(dirs::data_dir())
}

/// The one place either platform arm's candidate is turned into policy: a
/// candidate that is empty or relative is no durable place either — an
/// empty path names nothing, and a relative one would resolve against the
/// process's CWD rather than any platform-supplied location — so both are
/// refused here, once, for every platform (adr-0016, "Degrade loudly, or
/// not at all").
#[cfg(not(target_arch = "wasm32"))]
fn resolve_data_dir(candidate: Option<PathBuf>) -> Option<PathBuf> {
    candidate
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join("kayzen"))
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
fn platform_habit_repository() -> Option<PersistentHabitRepository> {
    use crate::infrastructure::local_storage_snapshot_store::LocalStorageSnapshotStore;

    Some(PersistentHabitRepository::hydrated_from(
        Rc::new(LocalStorageSnapshotStore::at("kayzen.habits.v1")),
        Rc::new(LocalStorageSnapshotStore::at("kayzen.habits.unreadable.v1")),
    ))
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
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock must be after the epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("kayzen-composition-test-{nanos}-{unique}"))
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

    // @scenario: persistence/S4
    #[test]
    fn an_oversized_primary_file_is_preserved_at_a_refused_sibling_before_the_next_save_overwrites_it()
     {
        use crate::infrastructure::file_snapshot_store::FileSnapshotStore;

        let dir = unused_temp_dir();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let primary = dir.join("habits.json");
        let file = fs::File::create(&primary).expect("temp file must be creatable");
        file.set_len(FileSnapshotStore::MAX_PAYLOAD_BYTES + 1)
            .expect("sparse file must be extendable");

        let services = Services::with_repository(Rc::new(persistent_habit_repository_at(&dir)));
        assert!(
            services.list_board_habits.handle().is_empty(),
            "an oversized primary must hydrate to an empty board, not a crash"
        );
        services
            .add_habit
            .execute("Boire de l'eau".to_string(), STARTING_GOAL)
            .expect("an empty board has room for one habit");

        assert!(
            fs::metadata(&primary)
                .map(|m| m.len() <= FileSnapshotStore::MAX_PAYLOAD_BYTES)
                .unwrap_or(false),
            "expected the fresh save to replace the primary with a small snapshot"
        );
        assert!(
            dir.join("habits.json.refused").exists(),
            "expected the oversized original preserved at a sibling path"
        );
    }

    #[test]
    fn default_data_dir_is_named_kayzen() {
        let dir = default_data_dir().expect("CI platforms always resolve a data directory");

        assert_eq!(dir.file_name(), Some(std::ffi::OsStr::new("kayzen")));
    }

    #[test]
    fn default_data_dir_stays_within_the_platforms_data_directory() {
        let dir = default_data_dir().expect("CI platforms always resolve a data directory");

        assert!(
            dir.starts_with(
                dirs::data_dir().expect("CI platforms always resolve a data directory")
            ),
            "expected the resolved directory to live under the platform's own data directory, got {dir:?}"
        );
    }

    #[test]
    fn resolve_data_dir_appends_kayzen_to_the_given_directory() {
        let base = PathBuf::from("/some/base");

        assert_eq!(
            resolve_data_dir(Some(base.clone())),
            Some(base.join("kayzen"))
        );
    }

    // @scenario: persistence/S5
    #[test]
    fn resolve_data_dir_is_none_when_the_platform_has_no_data_directory() {
        assert_eq!(resolve_data_dir(None), None);
    }

    // @scenario: persistence/S5
    #[test]
    fn resolve_data_dir_is_none_for_an_empty_candidate() {
        assert_eq!(resolve_data_dir(Some(PathBuf::from(""))), None);
    }

    // @scenario: persistence/S5
    //
    // Named for why, not just what: a relative candidate is refused rather
    // than joined, because joining it would resolve against the process's
    // current working directory instead of any platform-supplied location —
    // exactly the "somewhere else" adr-0016 refuses to invent.
    #[test]
    fn resolve_data_dir_is_none_for_a_relative_candidate_because_joining_it_would_resolve_against_the_process_cwd()
     {
        assert_eq!(resolve_data_dir(Some(PathBuf::from("files/kayzen"))), None);
    }

    // @scenario: persistence/S5
    //
    // Only pins `repository.is_none()`. A prior version of this test also
    // asserted a fresh temp path stayed absent, but that path was never
    // passed to `platform_habit_repository_from` — `Option::map` on `None`
    // never runs its closure, so no `FileSnapshotStore` is ever built and
    // "touches no disk" held by construction, not by anything this test
    // exercised. S5's "nothing is written to disk" is pinned by the
    // `is_none()` assertion below (no repository built ⇒ nothing to write
    // through) together with `load_over_an_absent_path_attempts_no_rename`
    // in `file_snapshot_store.rs`, which proves the store itself creates no
    // directory when there is nothing to preserve.
    #[test]
    fn platform_habit_repository_from_none_builds_no_repository() {
        let repository = platform_habit_repository_from(None);

        assert!(
            repository.is_none(),
            "no data directory means no repository can be built"
        );
    }
}
