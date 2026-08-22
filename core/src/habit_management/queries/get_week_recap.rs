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
    pub minutes_practised: u32,
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
    pub starting_goal: u32,
    pub current_goal: u32,
    pub steps: Vec<u32>,
    pub practised_recently: bool,
}

impl HabitProgress {
    fn for_habit(habit: &Habit, today: LocalDate) -> HabitProgress {
        let changes = habit.step_history().changes();
        HabitProgress {
            title: habit.title().value().to_string(),
            starting_goal: changes[0].goal().value(),
            current_goal: habit.current_goal(),
            steps: changes.iter().map(|change| change.goal().value()).collect(),
            practised_recently: practised_recently(habit, today),
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

impl GetWeekRecap {
    pub fn new(repository: Rc<dyn HabitRepository>, clock: Rc<dyn Clock>) -> GetWeekRecap {
        GetWeekRecap { repository, clock }
    }

    pub fn handle(&self) -> WeekRecap {
        let today = self.clock.today();
        let habits = self.repository.all();

        let mut minutes_practised = 0u32;
        for habit in &habits {
            minutes_practised = minutes_practised.saturating_add(habit.minutes_practised(today));
        }

        let rhythm = rhythm_for(&habits, today);
        let recently = rhythm.iter().any(|&practised| practised);

        WeekRecap {
            minutes_practised,
            habits: habits
                .iter()
                .map(|habit| HabitProgress::for_habit(habit, today))
                .collect(),
            rhythm,
            message: message_for(minutes_practised, recently),
        }
    }
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

/// Practised at least once in the rolling window ending today (same window,
/// same constant, as `rhythm_for`). A dedicated function, not inlined into
/// `for_habit`: `HabitProgress` derives no `Default`, so cargo-mutants
/// classes `for_habit` itself `unviable` and measures nothing there — a
/// distinct `-> bool` function receives viable mutants the tests above kill.
fn practised_recently(habit: &Habit, today: LocalDate) -> bool {
    (0..ROLLING_WINDOW_DAYS).any(|days_back| habit.is_done_on(today.minus_days(days_back)))
}

fn message_for(minutes_practised: u32, practised_recently: bool) -> WeekMessage {
    if minutes_practised == 0 {
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
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit::Habit;
    use crate::habit_management::domain::habit_id::HabitId;
    use crate::habit_management::domain::habit_repository::HabitRepository;
    use crate::habit_management::domain::habit_title::HabitTitle;
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
    // - the figure sums minutes practised across every habit (S1).
    // - paused and anchored habits still count in the sum (S2, sum half only —
    //   the row-per-habit half of the same scenario belongs to task 3).
    // - no practice yet -> message FreshStart, figure at 0 (S3).
    // - practised earlier, not in the last 7 days -> message Resting (S4).
    // - practised within the last 7 days -> message Growing (branch
    //   completeness: no scenario names this arm on its own, it is the third
    //   arm the message rule already states in the tech spec).
    // - the running sum saturates across habits instead of overflowing.
    // - each habit's row reads its journey with one bar per goal step, not
    //   one per completed day (S5).
    // - a brand-new, not-yet-practised habit's row still reads a journey,
    //   starting_goal == current_goal, a single step (S7).
    // - every habit gets a row, whatever its lifecycle state — paused and
    //   anchored habits included (S2, row half).
    // - the rhythm keeps one dot per day over the rolling 7-day window,
    //   oldest first, lit when at least one habit was practised that day —
    //   OR'd across every habit, not just the first one (S6).
    // - an empty repository (a new user, before any habit exists) never
    //   panics: no rows, all seven rhythm dots unlit, message FreshStart.
    //   Unanchored — no scenario in week-recap.feature names this boundary;
    //   authoring one is the PM's lane, not the developer's.
    // - a habit practised today reads practised_recently == true (S8).
    // - a habit last practised six days back (the last day still inside the
    //   rolling window) still reads practised_recently == true — unanchored,
    //   technical boundary.
    // - a habit last practised seven days back (the first day already
    //   outside the window) reads practised_recently == false — unanchored,
    //   technical boundary; paired with the six-days-back case above, this
    //   is what kills an off-by-one on the window's edge.
    // - a habit never practised reads practised_recently == false (S8).

    // @scenario: week-recap/S1
    #[test]
    fn the_figure_sums_minutes_practised_across_every_habit() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        for (id, days_done) in [("h-1", 3), ("h-2", 2), ("h-3", 1)] {
            let mut habit = a_habit(id, 5, TODAY - 6);
            for days_back in 0..days_done {
                habit.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
            }
            repository.save(&habit);
        }
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.minutes_practised, 30,
            "3 + 2 + 1 completed days at 5 minutes each sum to 30"
        );
    }

    // @scenario: week-recap/S2
    #[test]
    fn paused_and_anchored_habits_still_count_in_the_sum() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut paused = a_habit("h-1", 5, TODAY - 6);
        for days_back in 0..4 {
            paused.toggle_done(LocalDate::from_epoch_day(TODAY - 3 - days_back));
        }
        paused.pause().expect("a fresh habit is active");
        repository.save(&paused);

        let mut anchored = a_habit("h-2", 5, TODAY - 6);
        for days_back in 0..3 {
            anchored.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
        }
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);

        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.minutes_practised, 35,
            "4 days at 5 min from the paused habit plus 3 from the anchored \
             one — pausing or anchoring never takes lived minutes back"
        );
        assert_eq!(
            recap.habits.len(),
            2,
            "each habit still reads its own journey as a row, whatever its \
             lifecycle state"
        );
        assert!(
            recap.habits.iter().all(|habit| habit
                == &HabitProgress {
                    title: "Lire une page".to_string(),
                    starting_goal: 5,
                    current_goal: 5,
                    steps: vec![5],
                    practised_recently: true,
                }),
            "neither pausing nor anchoring must filter a habit out of its own \
             row or change how its journey reads: got {:?}",
            recap.habits
        );
    }

    // @scenario: week-recap/S3
    #[test]
    fn a_week_with_no_practice_reads_as_a_fresh_start() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5, TODAY));
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.minutes_practised, 0);
        assert_eq!(recap.message, WeekMessage::FreshStart);
    }

    // @scenario: week-recap/S4
    #[test]
    fn practice_before_the_last_seven_days_is_acknowledged_as_rest() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 20);
        habit.toggle_done(LocalDate::from_epoch_day(TODAY - 10));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.minutes_practised, 5,
            "the lived minutes are never erased by rest"
        );
        assert_eq!(recap.message, WeekMessage::Resting);
    }

    #[test]
    fn practice_within_the_last_seven_days_reads_as_growing() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 2);
        habit.toggle_done(LocalDate::from_epoch_day(TODAY));
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
            habit.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
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
        habit.toggle_done(LocalDate::from_epoch_day(TODAY + 5));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(
            recap.minutes_practised, 5,
            "the creation day's practice must be counted even when the \
             clock's today lags behind created_on, never read as zero"
        );
    }

    #[test]
    fn the_running_sum_saturates_instead_of_overflowing_across_habits() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        for id in ["h-1", "h-2"] {
            let mut habit = a_habit(id, u32::MAX, TODAY);
            habit.toggle_done(LocalDate::from_epoch_day(TODAY));
            repository.save(&habit);
        }
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.minutes_practised, u32::MAX);
    }

    // @scenario: week-recap/S5
    #[test]
    fn each_habit_shows_its_journey_with_one_bar_per_goal_step() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 3, TODAY - 6);
        habit.grow(LocalDate::from_epoch_day(TODAY - 4));
        habit.grow(LocalDate::from_epoch_day(TODAY - 2));
        for days_back in 0..4 {
            habit.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
        }
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert_eq!(recap.habits.len(), 1);
        assert_eq!(
            recap.habits[0],
            HabitProgress {
                title: "Lire une page".to_string(),
                starting_goal: 3,
                current_goal: 5,
                steps: vec![3, 4, 5],
                practised_recently: true,
            },
            "three goal steps were recorded but four days were completed — \
             the curve must draw one bar per step (3), not one per completed \
             day (4)"
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
                starting_goal: 5,
                current_goal: 5,
                steps: vec![5],
                practised_recently: false,
            },
            "an empty start is still a start — a single step, its starting \
             and current goal equal"
        );
    }

    // @scenario: week-recap/S8
    #[test]
    fn a_habit_practised_today_reads_as_practised_recently() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 6);
        habit.toggle_done(LocalDate::from_epoch_day(TODAY));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert!(
            recap.habits[0].practised_recently,
            "a habit practised today must read as practised in the rolling \
             window, got {:?}",
            recap.habits[0]
        );
    }

    #[test]
    fn a_habit_last_practised_six_days_back_still_reads_as_practised_recently() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 20);
        habit.toggle_done(LocalDate::from_epoch_day(TODAY - 6));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert!(
            recap.habits[0].practised_recently,
            "six days back is still inside the rolling seven-day window, \
             got {:?}",
            recap.habits[0]
        );
    }

    #[test]
    fn a_habit_last_practised_seven_days_back_no_longer_reads_as_practised_recently() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", 5, TODAY - 20);
        habit.toggle_done(LocalDate::from_epoch_day(TODAY - 7));
        repository.save(&habit);
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert!(
            !recap.habits[0].practised_recently,
            "seven days back is already outside the rolling seven-day \
             window, got {:?}",
            recap.habits[0]
        );
    }

    // @scenario: week-recap/S8
    #[test]
    fn a_never_practised_habit_does_not_read_as_practised_recently() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", 5, TODAY));
        let query = get_week_recap_over(repository);

        let recap = query.handle();

        assert!(
            !recap.habits[0].practised_recently,
            "a habit never practised must not read as practised, got {:?}",
            recap.habits[0]
        );
    }

    // @scenario: week-recap/S6
    #[test]
    fn the_rhythm_keeps_one_dot_per_day_lit_when_any_habit_was_practised() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit_a = a_habit("h-1", 5, TODAY - 6);
        habit_a.toggle_done(LocalDate::from_epoch_day(TODAY - 6));
        habit_a.toggle_done(LocalDate::from_epoch_day(TODAY - 2));
        repository.save(&habit_a);
        let mut habit_b = a_habit("h-2", 5, TODAY - 6);
        habit_b.toggle_done(LocalDate::from_epoch_day(TODAY - 4));
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

        assert_eq!(recap.minutes_practised, 0);
        assert!(recap.habits.is_empty(), "got {:?}", recap.habits);
        assert_eq!(recap.rhythm, vec![false; 7]);
        assert_eq!(recap.message, WeekMessage::FreshStart);
    }
}
