use std::rc::Rc;

use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::habit_management::domain::lifecycle_state::LifecycleState;
use crate::shared::clock::Clock;
use crate::shared::local_date::LocalDate;

#[derive(Clone)]
pub struct GetHabitDetail {
    repository: Rc<dyn HabitRepository>,
    clock: Rc<dyn Clock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HabitDetail {
    pub id: String,
    pub title: String,
    pub current_goal: u32,
    pub next_goal_up: u32,
    pub next_goal_down: u32,
    pub days: Vec<PracticeDay>,
    pub state: HabitState,
}

/// The habit's lifecycle as seen by the detail screen — a DTO-side type, kept
/// distinct from the domain's `LifecycleState` (adr-0006, adr-0010: the app
/// crate never imports a domain type) and never a `bool` (adr-0007: a second
/// bool for `Anchored` one slice from now would make an impossible
/// combination representable).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HabitState {
    Active,
    Paused,
    Anchored,
}

/// How many calendar days the practice staircase covers. Seven, aligned with
/// the week recap's own rhythm — the owner took the trade-off knowingly: a
/// longer window would show the effort trend more strongly, legibility won
/// (lifecycle-backlog, slice 3b).
const WINDOW_DAYS: i64 = 7;

/// One calendar day of the practice staircase: whether the habit was done that
/// day, and the goal that was active on it.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticeDay {
    pub done: bool,
    pub goal: u32,
}

impl GetHabitDetail {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> GetHabitDetail {
        GetHabitDetail { repository, clock }
    }

    pub fn handle(&self, habit_id: &str) -> Option<HabitDetail> {
        let id = HabitId::new(habit_id).ok()?;
        let habit = self.repository.get(&id)?;
        let today = self.clock.today();

        Some(HabitDetail {
            id: habit.id().value().to_string(),
            title: habit.title().value().to_string(),
            current_goal: habit.current_goal(),
            next_goal_up: habit.step_history().current().grown().value(),
            next_goal_down: habit.step_history().current().lightened().value(),
            days: (0..WINDOW_DAYS)
                .rev()
                .map(|days_back| today.minus_days(days_back))
                .map(|day| PracticeDay {
                    done: habit.is_done_on(day),
                    goal: goal_active_on(&habit, day),
                })
                .collect(),
            state: match habit.state() {
                LifecycleState::Active => HabitState::Active,
                LifecycleState::Paused => HabitState::Paused,
                LifecycleState::Anchored => HabitState::Anchored,
            },
        })
    }
}

/// The goal a habit was aiming at on `day`: the last step dated on or before it.
///
/// A day older than the habit itself falls back to the goal it started on. The
/// bar is faint there anyway, and standing it at zero would punch exactly the
/// hole the faint bar exists to avoid — an empty start is still a start
/// (practice-staircase/S6).
///
/// Indexing the first step cannot panic: `StepHistory::seeded` is its only
/// constructor, so a history always holds at least the step it was seeded with.
fn goal_active_on(habit: &Habit, day: LocalDate) -> u32 {
    let steps = habit.step_history().changes();

    steps
        .iter()
        .rev()
        .find(|step| step.on() <= day)
        .unwrap_or(&steps[0])
        .goal()
        .value()
}

#[cfg(test)]
mod tests {
    use super::{GetHabitDetail, HabitDetail, HabitState, PracticeDay};
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::habit_repository::HabitRepository;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::shared::clock::{Clock, FixedClock};
    use crate::shared::local_date::LocalDate;
    use std::rc::Rc;

    const CREATED_ON: i64 = 19_990;
    const TODAY: i64 = 20_000;

    fn a_habit() -> Habit {
        Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn seven_days(done: bool, goal: u32) -> Vec<PracticeDay> {
        vec![PracticeDay { done, goal }; 7]
    }

    fn get_habit_detail_over(repository: Rc<InMemoryHabitRepository>) -> GetHabitDetail {
        GetHabitDetail::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(TODAY))) as Rc<dyn Clock>,
        )
    }

    // Test List — GetHabitDetail (@feature:adjust-goal, staircase display — the
    // scenario gap noted under "Open questions"):
    // - a known habit id returns its detail: title, current goal, one-step staircase.
    // - an unknown habit id returns None (stale URL / deleted habit, d2).
    // @scenario: adjust-goal/S6
    #[test]
    fn a_known_habit_returns_its_title_goal_and_staircase() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        let query = get_habit_detail_over(Rc::clone(&repository));

        let result = query.handle("h-1");

        assert_eq!(
            result,
            Some(HabitDetail {
                id: "h-1".to_string(),
                title: "Read one page".to_string(),
                current_goal: 5,
                next_goal_up: 6,
                next_goal_down: 4,
                days: seven_days(false, 5),
                state: HabitState::Active,
            })
        );
    }

    // @scenario: adjust-goal/S7
    #[test]
    fn a_habit_at_the_floor_offers_lightening_toward_the_same_floor() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(1).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        ));
        let query = get_habit_detail_over(Rc::clone(&repository));

        let result = query.handle("h-1");

        assert_eq!(result.map(|detail| detail.next_goal_down), Some(1));
    }

    // No Gherkin scenario names this path yet (d2, stale URL / deleted habit) —
    // flagged under "Open questions" rather than authoring one (PM's lane).
    #[test]
    fn an_unknown_habit_id_returns_none() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let query = get_habit_detail_over(repository);

        assert_eq!(query.handle("missing"), None);
    }

    // No Gherkin scenario names this path yet either (invalid-id refusal,
    // T1 conformance with adr-0001) — flagged under "Open questions".
    #[test]
    fn an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        let query = get_habit_detail_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        assert_eq!(query.handle(&too_long), None);
    }

    // Test List — the practice staircase (@feature:practice-staircase). One bar
    // per calendar day, never one per goal change: the drawing credits practice,
    // not intent (lifecycle-backlog, slice 3b).
    // - the window is always seven days, whatever the habit's age or activity.
    // - a day that was done draws a full bar at that day's goal.
    // - a day that was not done draws the same bar, faintly.
    // - adjusting the goal draws nothing on its own.
    // @scenario: practice-staircase/S5
    #[test]
    fn a_habit_shows_one_bar_for_each_of_the_last_seven_days() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        let query = get_habit_detail_over(Rc::clone(&repository));

        let result = query.handle("h-1");

        assert_eq!(result.unwrap().days.len(), 7);
    }

    // @scenario: practice-staircase/S1
    #[test]
    fn a_day_that_was_done_draws_a_full_bar_at_that_days_goal() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.toggle_done(LocalDate::from_epoch_day(TODAY));
        repository.save(&habit);
        let query = get_habit_detail_over(Rc::clone(&repository));

        let days = query.handle("h-1").unwrap().days;

        assert_eq!(
            days[6],
            PracticeDay {
                done: true,
                goal: 5
            }
        );
    }

    // @scenario: practice-staircase/S4
    #[test]
    fn each_bar_stands_at_the_goal_that_was_active_that_day() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.toggle_done(LocalDate::from_epoch_day(TODAY - 1));
        habit.grow(LocalDate::from_epoch_day(TODAY));
        habit.toggle_done(LocalDate::from_epoch_day(TODAY));
        repository.save(&habit);
        let query = get_habit_detail_over(Rc::clone(&repository));

        let days = query.handle("h-1").unwrap().days;

        assert_eq!(
            (days[5].goal, days[6].goal),
            (5, 6),
            "the earlier day keeps the goal it was practised at; growing \
             afterwards raises only the days that follow"
        );
    }

    // @scenario: practice-staircase/S6
    #[test]
    fn a_brand_new_habit_already_shows_a_full_window_of_faint_bars() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(TODAY),
        ));
        let query = get_habit_detail_over(Rc::clone(&repository));

        let days = query.handle("h-1").unwrap().days;

        assert_eq!(
            days,
            seven_days(false, 5),
            "the days older than the habit stand at the goal it started on — \
             an empty start is still a start, and a zero-height bar would be \
             the hole the faint bar exists to avoid"
        );
    }

    // @scenario: practice-staircase/S3
    #[test]
    fn adjusting_the_goal_draws_no_new_bar_and_relives_no_day() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.toggle_done(LocalDate::from_epoch_day(TODAY - 2));
        repository.save(&habit);
        let query = get_habit_detail_over(Rc::clone(&repository));
        let before = query.handle("h-1").unwrap().days;

        habit.grow(LocalDate::from_epoch_day(TODAY));
        habit.grow(LocalDate::from_epoch_day(TODAY));
        habit.grow(LocalDate::from_epoch_day(TODAY));
        repository.save(&habit);

        let after = query.handle("h-1").unwrap().days;

        assert_eq!(
            after.len(),
            before.len(),
            "three taps on grandir add no bar: the staircase draws practice, \
             not intent"
        );
        assert_eq!(
            after.iter().map(|day| day.done).collect::<Vec<_>>(),
            before.iter().map(|day| day.done).collect::<Vec<_>>(),
            "and no day becomes lived by deciding"
        );
        assert_eq!(
            after[..6],
            before[..6],
            "the days already lived keep the height they were lived at"
        );
    }

    // @scenario: practice-staircase/S2
    #[test]
    fn a_day_without_practice_still_draws_its_bar() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.toggle_done(LocalDate::from_epoch_day(TODAY));
        repository.save(&habit);
        let query = get_habit_detail_over(Rc::clone(&repository));

        let days = query.handle("h-1").unwrap().days;

        assert!(days[6].done, "today was practised");
        assert!(
            !days[5].done,
            "yesterday was not — its bar is still drawn, only faint: a day \
             without practice is never a gap and never a warning"
        );
    }

    type Mutation = fn(&mut Habit);

    // The DTO-side state is mapped from the domain's LifecycleState by an
    // exhaustive match (adr-0007 AD-2). One behavior, three divergent rows.
    #[test]
    fn a_habits_state_is_mapped_from_its_lifecycle() {
        let cases: Vec<(Mutation, HabitState)> = vec![
            (|_habit| {}, HabitState::Active),
            (
                |habit: &mut Habit| {
                    habit.pause().expect("a fresh habit is active");
                },
                HabitState::Paused,
            ),
            (
                |habit: &mut Habit| {
                    habit.anchor().expect("a fresh habit is active");
                },
                HabitState::Anchored,
            ),
        ];

        for (mutate, expected) in cases {
            let repository = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit();
            mutate(&mut habit);
            repository.save(&habit);
            let query = get_habit_detail_over(repository);

            let result = query.handle("h-1");

            assert_eq!(result.map(|detail| detail.state), Some(expected));
        }
    }
}
