use std::rc::Rc;

use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::shared::clock::Clock;
use crate::shared::local_date::LocalDate;

/// The rolling window: both the word's *recently* and the rhythm row's dot
/// count read it (adr-0006: a rolling window ending today, never a
/// Monday→Sunday calendar week).
const ROLLING_WINDOW_DAYS: i64 = 7;

#[derive(Clone)]
pub struct GetWeekRecap {
    repository: Rc<dyn HabitRepository>,
    clock: Rc<dyn Clock>,
}

/// The Week screen's per-screen read model (adr-0006): everything derived on
/// read from every habit's histories, whatever its lifecycle state (AD-3).
#[derive(Debug, Clone, PartialEq)]
pub struct WeekRecap {
    pub times_practised: u32,
    pub habits: Vec<HabitProgress>,
    pub rhythm: Vec<bool>,
    pub message: WeekMessage,
}

/// One habit's journey this week, whatever its lifecycle state (AD-3). No
/// `id` and no `state`: the row carries no gesture (no link, no button, no
/// navigation), so it needs neither — an `id` on a row is identity granted
/// by a gesture the row does not have (adr-0006, same precedent as
/// `AnchoredHabit`).
#[derive(Debug, Clone, PartialEq)]
pub struct HabitProgress {
    pub title: String,
    pub from_goal: Option<u32>,
    pub current_goal: u32,
    pub practised_day_goals: Vec<u32>,
}

impl HabitProgress {
    fn for_habit(habit: &Habit, today: LocalDate) -> HabitProgress {
        let current_goal = habit.current_goal();
        let window_start = today.minus_days(ROLLING_WINDOW_DAYS - 1);
        let goal_at_window_start = habit.step_history().goal_on(window_start).value();
        HabitProgress {
            title: habit.title().value().to_string(),
            // @algo: the arrow is the net change over the window (goal in force at its first day vs today's), never the lifetime starting goal; a younger habit falls back to its creation goal.
            from_goal: Some(goal_at_window_start).filter(|&goal| goal != current_goal),
            current_goal,
            practised_day_goals: practised_day_goals_for(habit, today),
        }
    }
}

/// How the week is living right now, as the recap says it — a DTO-side enum
/// (adr-0006: a domain-shaped choice crosses as an enum, never a bool, and the
/// French words live in the view, never in core).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeekMessage {
    FreshStart,
    Resting,
    Growing,
}

impl WeekMessage {
    pub const ALL: [WeekMessage; 3] = [
        WeekMessage::FreshStart,
        WeekMessage::Resting,
        WeekMessage::Growing,
    ];
}

impl GetWeekRecap {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> GetWeekRecap {
        GetWeekRecap { repository, clock }
    }

    pub fn handle(&self) -> WeekRecap {
        let today = self.clock.today();
        let habits = self.repository.all();

        let times_practised = habits.iter().fold(0u32, |total, habit| {
            total.saturating_add(attempts_in_window(habit, today))
        });
        // @algo: the figure reads this week only, but a practice before the window still tells "rest" from "fresh start".
        let ever_practised = habits.iter().any(|habit| habit.times_practised(today) > 0);

        let rhythm = rhythm_for(&habits, today);
        let recently = rhythm.iter().any(|&practised| practised);

        WeekRecap {
            times_practised,
            habits: habits
                .iter()
                .map(|habit| HabitProgress::for_habit(habit, today))
                .collect(),
            rhythm,
            message: message_for(ever_practised, recently),
        }
    }
}

// @algo: every attempt over the rolling window ending today, the same window and constant as `rhythm_for`.
fn attempts_in_window(habit: &Habit, today: LocalDate) -> u32 {
    (0..ROLLING_WINDOW_DAYS).fold(0u32, |total, days_back| {
        total.saturating_add(habit.attempts_on(today.minus_days(days_back)))
    })
}

/// One dot per day over the rolling window ending today, oldest first, lit
/// when at least one habit was practised that day (AD-4: presence, never a
/// density/ratio — `LifecycleState` carries no history, so a denominator
/// taken from today's habit set would rewrite the past whenever the user
/// pauses or anchors something).
fn rhythm_for(habits: &[Habit], today: LocalDate) -> Vec<bool> {
    (0..ROLLING_WINDOW_DAYS)
        .rev()
        .map(|days_back| {
            let day = today.minus_days(days_back);
            habits.iter().any(|habit| habit.is_done_on(day))
        })
        .collect()
}

/// One entry per day actually practised within the rolling window ending
/// today (same window, same constant, as `rhythm_for`), oldest first, each
/// carrying the goal that was in force on that day. A dedicated function,
/// not inlined into `for_habit`: `HabitProgress` derives no `Default`, so
/// cargo-mutants classes `for_habit` itself `unviable` and measures nothing
/// there — a distinct `-> Vec<u32>` function receives viable mutants the
/// tests above kill.
fn practised_day_goals_for(habit: &Habit, today: LocalDate) -> Vec<u32> {
    (0..ROLLING_WINDOW_DAYS)
        .rev()
        .map(|days_back| today.minus_days(days_back))
        .filter(|&day| habit.is_done_on(day))
        .map(|day| habit.step_history().goal_on(day).value())
        .collect()
}

fn message_for(ever_practised: bool, practised_recently: bool) -> WeekMessage {
    if !ever_practised {
        return WeekMessage::FreshStart;
    }
    if practised_recently {
        return WeekMessage::Growing;
    }
    WeekMessage::Resting
}

#[cfg(test)]
mod tests {
    use super::{GetWeekRecap, HabitProgress, WeekMessage};
    use crate::habit_management::domain::completion_history::CompletionHistory;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::habit_repository::HabitRepository;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::domain::lifecycle_state::LifecycleState;
    use crate::habit_management::domain::step_history::StepHistory;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::shared::clock::{Clock, FixedClock};
    use crate::shared::local_date::LocalDate;
    use std::rc::Rc;

    const TODAY: i64 = 20_000;

    fn a_habit(id: &str, goal: u32, created_on: i64) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(goal).unwrap(),
            LocalDate::from_epoch_day(created_on),
        )
    }

    fn get_week_recap_over(repository: Rc<InMemoryHabitRepository>) -> GetWeekRecap {
        GetWeekRecap::new(
            repository as Rc<dyn HabitRepository>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(TODAY))) as Rc<dyn Clock>,
        )
    }

    // Test List — GetWeekRecap (@feature:week-recap):
    // - paused and anchored habits still count in the sum, and each still
    //   reads its own journey as a row, whatever its lifecycle state (S2).
    // - no practice yet -> message FreshStart, figure at 0 (S3).
    // - practised earlier, not in the last 7 days -> message Resting (S4).
    // - practised within the last 7 days -> message Growing (branch
    //   completeness: no scenario names this arm on its own, it is the third
    //   arm the message rule already states in the tech spec).
    // - the running sum saturates across habits instead of overflowing.
    // - each habit's mini-curve draws one bar per day practised, never one
    //   per recorded goal step, each bar carrying the goal in force on the
    //   day it draws (S5).
    // - practised days read oldest first, never reversed — unanchored,
    //   technical boundary (kills an order mutant no scenario names).
    // - a habit never practised in the window draws no bars at all: an empty
    //   start is still a start, its journey line stands on its own (S7).
    // - every habit gets a row, whatever its lifecycle state — paused and
    //   anchored habits included (S2, row half).
    // - the rhythm keeps one dot per day over the rolling 7-day window,
    //   oldest first, lit when at least one habit was practised that day —
    //   OR'd across every habit, not just the first one (S6).
    // - an empty repository (a new user, before any habit exists) never
    //   panics: no rows, all seven rhythm dots unlit, message FreshStart.
    //   Unanchored — no scenario in week-recap.feature names this boundary;
    //   authoring one is the PM's lane, not the developer's.
    // - only a habit practised at least once in the window draws bars; a
    //   habit never practised at all draws none (S8).
    // - the mini-curve reads the same rolling seven days as the rhythm: a
    //   last practice six days back still draws a bar, seven days back draws
    //   none (S9).

    // @scenario: week-recap/S1
    #[test]
    fn the_figure_sums_minutes_practised_across_every_habit() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        for (id, days_done) in [("h-1", 3), ("h-2", 2), ("h-3", 1)] {
            let mut habit = a_habit(id, 5, TODAY - 6);
            for days_back in 0..days_done {
                habit.record_attempt(LocalDate::from_epoch_day(TODAY - days_back));
            }
            repository.save(&habit);
        }
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.times_practised, 6,
            "3 + 2 + 1 completed days at one attempt each sum to 6 times"
        );
    }

    // @scenario: week-recap/S1
    #[test]
    fn every_attempt_of_a_day_counts_in_the_times_practised() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 6);
        for _ in 0..3 {
            habit.record_attempt(LocalDate::from_epoch_day(TODAY));
        }
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.times_practised, 3,
            "three attempts on one day are three times practised, never the \
             day's goal (5) and never one day"
        );
    }

    // @scenario: week-recap/S2
    #[test]
    fn paused_and_anchored_habits_still_count_in_the_sum() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut paused = a_habit("h-1", 5, TODAY - 6);
        for days_back in 0..4 {
            paused.record_attempt(LocalDate::from_epoch_day(TODAY - 3 - days_back));
        }
        paused.pause().expect("a fresh habit is active");
        repository.save(&paused);

        let mut anchored = a_habit("h-2", 5, TODAY - 6);
        for days_back in 0..3 {
            anchored.record_attempt(LocalDate::from_epoch_day(TODAY - days_back));
        }
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);

        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.times_practised, 7,
            "4 attempts from the paused habit plus 3 from the anchored \
             one — pausing or anchoring never takes lived times back"
        );
        assert_eq!(
            recap.habits.len(),
            2,
            "each habit still reads its own journey as a row, whatever its \
             lifecycle state"
        );
        assert_eq!(
            recap.habits[0],
            HabitProgress {
                title: "Lire une page".to_string(),
                from_goal: None,
                current_goal: 5,
                practised_day_goals: vec![5, 5, 5, 5],
            },
            "pausing must not filter the habit out of its own row or change \
             how its journey reads: got {:?}",
            recap.habits[0]
        );
        assert_eq!(
            recap.habits[1],
            HabitProgress {
                title: "Lire une page".to_string(),
                from_goal: None,
                current_goal: 5,
                practised_day_goals: vec![5, 5, 5],
            },
            "anchoring must not filter the habit out of its own row or \
             change how its journey reads: got {:?}",
            recap.habits[1]
        );
    }

    // @scenario: week-recap/S3
    #[test]
    fn a_week_with_no_practice_reads_as_a_fresh_start() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5, TODAY));
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.times_practised, 0);
        assert_eq!(recap.message, WeekMessage::FreshStart);
    }

    // @scenario: week-recap/S4
    #[test]
    fn practice_before_the_last_seven_days_is_acknowledged_as_rest() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 20);
        habit.record_attempt(LocalDate::from_epoch_day(TODAY - 10));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.times_practised, 0,
            "the figure reads this week only: nothing in the last seven days"
        );
        assert_eq!(
            recap.message,
            WeekMessage::Resting,
            "an earlier practice is never erased by rest: the word stays \
             Resting, never a fresh start"
        );
    }

    // @scenario: week-recap/S14
    #[test]
    fn the_figure_counts_only_the_attempts_of_the_rolling_window() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 20);
        for (days_back, attempts) in [(7, 2), (6, 1), (0, 2)] {
            for _ in 0..attempts {
                habit.record_attempt(LocalDate::from_epoch_day(TODAY - days_back));
            }
        }
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.times_practised, 3,
            "the 2 attempts seven days back are outside the window; the 1 \
             on its first day (six days back) and the 2 today are inside"
        );
    }

    #[test]
    fn practice_within_the_last_seven_days_reads_as_growing() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 2);
        habit.record_attempt(LocalDate::from_epoch_day(TODAY));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.message, WeekMessage::Growing);
    }

    // @scenario: week-recap/S4
    #[test]
    fn the_recent_practice_window_edge_sits_at_seven_days_back() {
        let cases: Vec<(i64, WeekMessage)> =
            vec![(6, WeekMessage::Growing), (7, WeekMessage::Resting)];

        for (days_back, expected) in cases {
            let repository = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 20);
            habit.record_attempt(LocalDate::from_epoch_day(TODAY - days_back));
            repository.save(&habit);
            let query = get_week_recap_over(repository);

            let recap = query.handle();

            assert_eq!(
                recap.message, expected,
                "a last completion {days_back} days back reads as {expected:?} — \
                 six is still within the rolling window, seven is already outside it"
            );
        }
    }

    // A device clock lagging behind a habit's own creation day (westward TZ
    // change, clock moved back) must still count the creation day's practice
    // — never silently read it as zero (AD-2's stated hazard).
    #[test]
    fn a_habit_created_in_the_clocks_future_still_counts_its_creation_day() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY + 5);
        habit.record_attempt(LocalDate::from_epoch_day(TODAY + 5));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.message,
            WeekMessage::Resting,
            "the creation day's practice must still be known even when the \
             clock's today lags behind created_on, never read as a fresh start"
        );
    }

    #[test]
    fn the_running_sum_saturates_instead_of_overflowing_across_habits() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        for id in ["h-1", "h-2"] {
            let habit = Habit::rehydrate(
                HabitId::new(id).unwrap(),
                HabitTitle::new("Lire une page".to_string()).unwrap(),
                StepHistory::seeded(LocalDate::from_epoch_day(TODAY), Goal::new(5).unwrap()),
                CompletionHistory::with_attempts(LocalDate::from_epoch_day(TODAY), u32::MAX),
                LifecycleState::Active,
            );
            repository.save(&habit);
        }
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.times_practised, u32::MAX);
    }

    // @scenario: week-recap/S5
    #[test]
    fn each_habits_mini_curve_draws_one_bar_per_day_practised() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 3, TODAY - 6);
        habit.grow(LocalDate::from_epoch_day(TODAY - 4));
        habit.grow(LocalDate::from_epoch_day(TODAY - 2));
        for days_back in 0..4 {
            habit.record_attempt(LocalDate::from_epoch_day(TODAY - days_back));
        }
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.habits.len(), 1);
        assert_eq!(
            recap.habits[0],
            HabitProgress {
                title: "Lire une page".to_string(),
                from_goal: Some(3),
                current_goal: 5,
                practised_day_goals: vec![4, 5, 5, 5],
            },
            "four days were practised (TODAY-3..=TODAY) — the curve must \
             draw one bar per day practised, each carrying the goal in \
             force that day, not one bar per recorded goal step (3)"
        );
    }

    // @scenario: week-recap/S12
    #[test]
    fn a_goal_raised_long_ago_shows_its_current_goal_with_no_from_goal() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 3, TODAY - 60);
        habit.grow(LocalDate::from_epoch_day(TODAY - 30));
        habit.grow(LocalDate::from_epoch_day(TODAY - 30));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            (recap.habits[0].from_goal, recap.habits[0].current_goal),
            (None, 5),
            "the goal moved before the window, so this week's row has no \
             arrow: got {:?}",
            recap.habits[0]
        );
    }

    // @scenario: week-recap/S5
    #[test]
    fn a_goal_changed_in_the_window_shows_the_goal_in_force_at_its_start() {
        let cases: Vec<(i64, Option<u32>)> = vec![(7, None), (6, None), (5, Some(3))];

        for (raised_days_back, expected) in cases {
            let repository = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 3, TODAY - 60);
            habit.grow(LocalDate::from_epoch_day(TODAY - raised_days_back));
            habit.grow(LocalDate::from_epoch_day(TODAY - raised_days_back));
            repository.save(&habit);
            let query = get_week_recap_over(repository);

            let recap = query.handle();

            assert_eq!(
                recap.habits[0].from_goal, expected,
                "a goal raised {raised_days_back} days back: the window opens \
                 six days back, so a raise on or before that day is already \
                 in force at its start and a later one is a change this week"
            );
        }
    }

    // @scenario: week-recap/S5
    #[test]
    fn a_goal_raised_then_lowered_back_inside_the_window_shows_no_from_goal() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 3, TODAY - 60);
        habit.grow(LocalDate::from_epoch_day(TODAY - 4));
        habit.grow(LocalDate::from_epoch_day(TODAY - 4));
        habit.lighten(LocalDate::from_epoch_day(TODAY - 2));
        habit.lighten(LocalDate::from_epoch_day(TODAY - 2));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            (recap.habits[0].from_goal, recap.habits[0].current_goal),
            (None, 3),
            "3 -> 5 -> 3 is no net change this week: got {:?}",
            recap.habits[0]
        );
    }

    // @scenario: week-recap/S5
    #[test]
    fn a_habit_created_inside_the_window_starts_from_its_creation_goal() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 3, TODAY - 2);
        habit.grow(LocalDate::from_epoch_day(TODAY - 1));
        habit.grow(LocalDate::from_epoch_day(TODAY - 1));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            (recap.habits[0].from_goal, recap.habits[0].current_goal),
            (Some(3), 5),
            "no earlier goal exists before creation: got {:?}",
            recap.habits[0]
        );
    }

    // Unanchored, technical boundary: no scenario names this order on its
    // own, but a reversed order would silently swap which day each bar's
    // height comes from.
    #[test]
    fn practised_day_goals_read_oldest_first() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 3, TODAY - 6);
        habit.record_attempt(LocalDate::from_epoch_day(TODAY - 5));
        habit.grow(LocalDate::from_epoch_day(TODAY - 4));
        habit.grow(LocalDate::from_epoch_day(TODAY - 3));
        habit.grow(LocalDate::from_epoch_day(TODAY - 2));
        habit.grow(LocalDate::from_epoch_day(TODAY - 1));
        habit.record_attempt(LocalDate::from_epoch_day(TODAY - 1));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.habits[0].practised_day_goals,
            vec![3, 7],
            "the older practised day (goal 3) must come first, the more \
             recent one (goal 7) last — never reversed, got {:?}",
            recap.habits[0]
        );
    }

    // @scenario: week-recap/S7
    #[test]
    fn a_brand_new_habit_already_shows_its_journey() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5, TODAY));
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.habits.len(), 1);
        assert_eq!(
            recap.habits[0],
            HabitProgress {
                title: "Lire une page".to_string(),
                from_goal: None,
                current_goal: 5,
                practised_day_goals: vec![],
            },
            "an empty start is still a start — its journey line reads on \
             its own, but no day has been practised yet so no bar is drawn"
        );
    }

    // @scenario: week-recap/S8
    #[test]
    fn only_a_habit_practised_in_the_window_draws_bars() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut practised = a_habit("h-1", 5, TODAY - 6);
        practised.record_attempt(LocalDate::from_epoch_day(TODAY));
        repository.save(&practised);
        repository.save(&a_habit("h-2", 5, TODAY));
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.habits[0].practised_day_goals,
            vec![5],
            "practised today must draw one bar, got {:?}",
            recap.habits[0]
        );
        assert!(
            recap.habits[1].practised_day_goals.is_empty(),
            "never practised must draw no bar at all, got {:?}",
            recap.habits[1]
        );
    }

    // @scenario: week-recap/S9
    #[test]
    fn the_mini_curve_reads_the_same_rolling_window_as_the_rhythm() {
        let cases: Vec<(i64, Vec<u32>)> = vec![(6, vec![5]), (7, vec![])];

        for (days_back, expected) in cases {
            let repository = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 20);
            habit.record_attempt(LocalDate::from_epoch_day(TODAY - days_back));
            repository.save(&habit);
            let query = get_week_recap_over(repository);

            let recap = query.handle();

            assert_eq!(
                recap.habits[0].practised_day_goals, expected,
                "a last practice {days_back} days back must draw {expected:?} \
                 — six is still within the rolling window the rhythm row \
                 also reads, seven is already outside it"
            );
        }
    }

    // @scenario: week-recap/S6
    #[test]
    fn the_rhythm_keeps_one_dot_per_day_lit_when_any_habit_was_practised() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit_a = a_habit("h-1", 5, TODAY - 6);
        habit_a.record_attempt(LocalDate::from_epoch_day(TODAY - 6));
        habit_a.record_attempt(LocalDate::from_epoch_day(TODAY - 2));
        repository.save(&habit_a);
        let mut habit_b = a_habit("h-2", 5, TODAY - 6);
        habit_b.record_attempt(LocalDate::from_epoch_day(TODAY - 4));
        repository.save(&habit_b);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.rhythm,
            vec![true, false, true, false, true, false, false],
            "seven dots, oldest first, lit on any day at least one habit was \
             practised — day one and five come from h-1, day three from h-2, \
             so the dot must be OR'd across every habit, not just the first"
        );
    }

    // Unanchored: characterizes the empty-repository boundary (a new user
    // opening the Week screen before creating any habit), which no scenario
    // in week-recap.feature names. Behaviour was already correct before this
    // test — it pins it so a future `for_habit`/indexing change cannot break
    // it silently.
    #[test]
    fn an_empty_repository_reads_as_a_fresh_start_with_no_rows() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.times_practised, 0);
        assert!(recap.habits.is_empty(), "got {:?}", recap.habits);
        assert_eq!(recap.rhythm, vec![false; 7]);
        assert_eq!(recap.message, WeekMessage::FreshStart);
    }
}
