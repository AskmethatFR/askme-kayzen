use crate::composition::Services;
use crate::i18n::{tr, tr_key};
use dioxus::prelude::*;
use kayzen_core::habit_management::queries::get_week_recap::WeekMessage;

/// The tinted card's staircase box, in the SVG's own user units, and the
/// radius of the pebble standing on the trace's last point.
const SPARK_WIDTH: f64 = 110.0;
const SPARK_HEIGHT: f64 = 56.0;
const SPARK_DOT_RADIUS: f64 = 5.0;

#[component]
pub fn Week() -> Element {
    let services = use_context::<Services>();
    let recap = use_signal(move || services.get_week_recap.handle());
    let recap = recap();
    let legend = tr!(
        "week-minutes-practised",
        count: recap.minutes_practised as i64
    );
    let staircase = spark_points(&recap.rhythm);
    let staircase_trace = staircase
        .iter()
        .map(|(x, y)| format!("{x:.2},{y:.2}"))
        .collect::<Vec<_>>()
        .join(" ");
    let staircase_end = staircase.last().copied();

    rsx! {
        div { class: "screen",
            h1 { class: "week-heading", {tr!("week-heading")} }

            div { class: "week-tint-card",
                div { class: "week-tint-figure",
                    span { class: "week-figure", "{recap.minutes_practised}" }
                    span { class: "week-figure-label", "{legend}" }
                }
                svg {
                    class: "week-spark",
                    view_box: "0 0 {SPARK_WIDTH} {SPARK_HEIGHT}",
                    "aria-hidden": "true",
                    "focusable": "false",
                    polyline { points: "{staircase_trace}" }
                    if let Some((end_x, end_y)) = staircase_end {
                        circle {
                            cx: "{end_x:.2}",
                            cy: "{end_y:.2}",
                            r: "{SPARK_DOT_RADIUS}",
                        }
                    }
                }
            }

            p { class: "week-word", {tr_key(week_copy_key(recap.message))} }

            div { class: "rhythm", "aria-label": tr!("week-rhythm-aria"),
                for (day_offset, practised) in recap.rhythm.iter().enumerate() {
                    span {
                        key: "{day_offset}",
                        class: if *practised { "rhythm-dot is-practised" } else { "rhythm-dot" },
                    }
                }
            }

            div { class: "week-habits",
                for (row_offset, habit) in recap.habits.iter().enumerate() {
                    div { key: "{row_offset}", class: "week-habit",
                        div { class: "week-habit-head",
                            p { class: "week-habit-title", "{habit.title}" }
                            p {
                                class: "week-habit-goal",
                                {tr!(
                                    "week-habit-goal",
                                    changed: if habit.current_goal == habit.starting_goal { "no" } else { "yes" },
                                    goal: habit.current_goal as i64,
                                    starting_goal: habit.starting_goal as i64,
                                    current_goal: habit.current_goal as i64
                                )}
                            }
                        }
                        if !habit.practised_day_goals.is_empty() {
                            div {
                                class: "week-curve",
                                "aria-label": tr!(
                                    "week-curve-aria",
                                    title: habit.title.clone(),
                                    starting_goal: habit.starting_goal as i64,
                                    current_goal: habit.current_goal as i64,
                                    practised_days: habit.practised_day_goals.len() as i64
                                ),
                                for (pebble_offset, ratio) in practice_ratios(&habit.practised_day_goals).into_iter().enumerate()
                                {
                                    span {
                                        key: "{pebble_offset}",
                                        class: "week-pebble",
                                        style: "--practice-ratio: {ratio}",
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The tinted card's staircase: the same seven booleans the rhythm row
/// draws, read oldest day first, stepping up once per day practised and
/// holding its level across a day of rest. Two points per day — the rise
/// and the flat run that follows it — so a rest day is a plateau, never a
/// fall. The vertical span is inset by the end pebble's radius (nothing is
/// clipped) and scaled by the window's own length, so seven practised days
/// climb to the top while seven days of rest leave one flat trace at the
/// bottom. Core hands over the booleans (adr-0010: core returns numbers,
/// the view decides how to draw them) — the shape is the view's call.
#[must_use]
fn spark_points(rhythm: &[bool]) -> Vec<(f64, f64)> {
    let days = rhythm.len() as f64;
    if days == 0.0 {
        return Vec::new();
    }

    let day_width = SPARK_WIDTH / days;
    let top = SPARK_DOT_RADIUS;
    let bottom = SPARK_HEIGHT - SPARK_DOT_RADIUS;
    let level = |practised_days: f64| bottom - (practised_days / days) * (bottom - top);

    let mut points = vec![(0.0, level(0.0))];
    let mut practised_days = 0.0;
    for (day_offset, practised) in rhythm.iter().enumerate() {
        if *practised {
            practised_days += 1.0;
            points.push((day_offset as f64 * day_width, level(practised_days)));
        }
        points.push(((day_offset as f64 + 1.0) * day_width, level(practised_days)));
    }

    points
}

/// Each pebble's size relative to its own row's tallest practised goal
/// (adr-0010: core returns numbers, the view decides how to draw them) —
/// never an absolute minute value. The owner's call: a 2→3 habit and a
/// 30→32 habit draw the same shape, because the row shows relative
/// progression, not absolute effort. `unwrap_or(1)` only guards an empty
/// slice; the view never calls this on an empty `practised_day_goals` (the
/// `.week-curve` is not rendered at all in that case), so it never actually
/// influences a returned ratio.
#[must_use]
fn practice_ratios(practised_day_goals: &[u32]) -> Vec<f64> {
    let row_max = practised_day_goals.iter().copied().max().unwrap_or(1) as f64;
    practised_day_goals
        .iter()
        .map(|&goal| goal as f64 / row_max)
        .collect()
}

#[must_use]
fn week_copy_key(message: WeekMessage) -> &'static str {
    match message {
        WeekMessage::FreshStart => "week-message-fresh-start",
        WeekMessage::Resting => "week-message-resting",
        WeekMessage::Growing => "week-message-growing",
    }
}

#[cfg(test)]
mod tests {
    use super::week_copy_key;
    use crate::composition::Services;
    use crate::i18n::{tr, use_locale_for_tests_as};
    use crate::route::Route;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use dioxus::prelude::*;
    use dioxus_i18n::unic_langid::langid;
    use kayzen_core::habit_management::domain::goal::Goal;
    use kayzen_core::habit_management::domain::habit::Habit;
    use kayzen_core::habit_management::domain::habit_id::HabitId;
    use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
    use kayzen_core::habit_management::domain::habit_title::HabitTitle;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use kayzen_core::habit_management::queries::get_week_recap::WeekMessage;
    use kayzen_core::shared::clock::Clock;
    use kayzen_core::shared::local_date::LocalDate;
    use std::rc::Rc;

    const TODAY: i64 = 20_000;

    struct FixedClock(LocalDate);

    impl Clock for FixedClock {
        fn today(&self) -> LocalDate {
            self.0
        }
    }

    #[test]
    fn every_week_copy_key_resolves_in_both_catalogues() {
        let (fr_ids, en_ids) = crate::i18n::catalogue_ids();

        for message in WeekMessage::ALL {
            let key = week_copy_key(message);
            assert!(fr_ids.contains(key), "expected {key} to resolve in fr.ftl");
            assert!(en_ids.contains(key), "expected {key} to resolve in en.ftl");
        }
    }

    fn a_habit(id: &str, goal: u32, created_on: i64) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(goal).unwrap(),
            LocalDate::from_epoch_day(created_on),
        )
    }

    fn services_with(repository: Rc<dyn HabitRepository>) -> Services {
        Services::with_repository_and_clock(
            repository,
            Rc::new(FixedClock(LocalDate::from_epoch_day(TODAY))),
        )
    }

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    #[component]
    fn RootAtWeekScreen() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            for (id, days_done) in [("h-1", 3), ("h-2", 2), ("h-3", 1)] {
                let mut habit = a_habit(id, 5, TODAY - 6);
                for days_back in 0..days_done {
                    habit.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
                }
                repository.save(&habit);
            }
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // The shared title lives at THIS fixture's own site, not borrowed from
    // a_habit -- a_habit hardcoding "Lire une page" for every caller is a
    // coincidence of a shared helper, not a property this regression test
    // may depend on (issue #30 retry-2: an edit to a_habit alone must never
    // silently defuse the collision below).
    const SHARED_TITLE: &str = "Lire une page";

    #[component]
    fn RootAtWeekScreenWithTwoRowsSharingATitle() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            for id in ["h-1", "h-2"] {
                repository.save(&Habit::new(
                    HabitId::new(id).unwrap(),
                    HabitTitle::new(SHARED_TITLE.to_string()).unwrap(),
                    Goal::new(5).unwrap(),
                    LocalDate::from_epoch_day(TODAY - 6),
                ));
            }
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // Test List — Week screen render (@feature:week-recap):
    // - the tinted card holds the figure and its legend apart: the figure
    //   prints the sum of minutes practised across every habit, the legend
    //   pluralises without reprinting it (S1).
    // - paused and anchored habits still count toward the figure (S2, sum half
    //   only — see get_week_recap.rs's test of the same name for the split).
    // - a fresh week reads its gentle word (S3).
    // - a week without recent practice reads rest, without blame (S4).
    // - the screen carries no masthead and no way back to Aujourd'hui: the
    //   bottom bar is the navigation (no scenario — the bar is chrome, see
    //   adr-0021).
    // - the tinted card's staircase is derived from the rhythm, aria-hidden,
    //   and ends on the last summit it reached (S10).
    // - each habit is a card whose line reads its goal at the right, and
    //   whose pebble band draws one pebble per day practised, not one per
    //   recorded goal step (S5), each pebble's size normalized to that
    //   row's own maximum practised goal, never an absolute minute value
    //   (owner decision, 2026-08-21: relative progression, not absolute
    //   effort).
    // - a never-practised habit's row draws no band at all — no empty
    //   container either — while its title and goal line still render (S7).
    // - a row lightened back down (its maximum practised goal sits
    //   mid-history, not last) still normalizes on that row's own maximum,
    //   never on its current goal — no pebble may exceed its container.
    // - the rhythm row shows seven dots, oldest first, lit on practiced days
    //   and faint on the rest, never a gap (S6).
    // - only a habit practised at least once in the rolling window draws a
    //   band at all; a habit not practised in the window gets nothing else
    //   added — no counter, no mark of absence (S8).
    // - the pebble band reads the same rolling seven days as the rhythm: a
    //   habit last practised six days back still draws a pebble, eight days
    //   back draws none (S9).
    // - the band's aria-label states how many days were practised, correct
    //   singular/plural, in both fr and en (language/S4).

    // @scenario: week-recap/S1
    #[test]
    fn the_week_screen_states_the_accumulated_minutes() {
        let html = render(RootAtWeekScreen);

        assert!(
            html.contains(r#"class="week-tint-card""#),
            "expected the figure to live on a tinted card, got: {html}"
        );
        assert!(
            html.contains(
                r#"<span class="week-figure">30</span><span class="week-figure-label">minutes de pratique accumulées</span>"#
            ),
            "expected the figure and its legend apart — the figure prints the \
             sum, the legend names accumulated practice and never gain over \
             the starting goal, got: {html}"
        );
        assert!(
            html.contains("Vous avancez"),
            "recent practice must read the Growing word, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithPausedAndAnchoredHabits() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
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
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: week-recap/S2 (sum half only — see get_week_recap.rs's test of
    // the same name for the row half, which belongs to task 3)
    #[test]
    fn paused_and_anchored_habits_still_count_toward_the_large_figure() {
        let html = render(RootAtWeekScreenWithPausedAndAnchoredHabits);

        assert!(
            html.contains(
                r#"<span class="week-figure">35</span><span class="week-figure-label">minutes de pratique accumulées</span>"#
            ),
            "pausing or anchoring must never take lived minutes back, got: {html}"
        );
    }

    #[component]
    fn RootAtFreshWeekScreen() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            repository.save(&a_habit("h-1", 5, TODAY));
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: week-recap/S3
    #[test]
    fn a_fresh_week_reads_a_gentle_word_that_is_never_a_bare_zero() {
        let html = render(RootAtFreshWeekScreen);

        assert!(
            html.contains("Un début parfait"),
            "expected the fresh-start word, an empty start is still a start, got: {html}"
        );
        assert!(
            html.contains(
                r#"<span class="week-figure">0</span><span class="week-figure-label">minutes de pratique accumulées</span>"#
            ),
            "a fresh week's figure is never a bare zero — it always carries \
             its label, got: {html}"
        );
    }

    #[component]
    fn RootAtRestingWeekScreen() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 20);
            habit.toggle_done(LocalDate::from_epoch_day(TODAY - 10));
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: week-recap/S4
    #[test]
    fn a_week_without_recent_practice_reads_as_rest_without_blame() {
        let html = render(RootAtRestingWeekScreen);

        assert!(
            html.contains("Cette semaine se repose"),
            "expected a resting word naming rest, never blame, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithOneMinute() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 1, TODAY);
            habit.toggle_done(LocalDate::from_epoch_day(TODAY));
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: week-recap/S1
    #[test]
    fn a_single_minute_reads_the_singular_form() {
        let html = render(RootAtWeekScreenWithOneMinute);

        assert!(
            html.contains(
                r#"<span class="week-figure">1</span><span class="week-figure-label">minute de pratique accumulée</span>"#
            ),
            "one minute must read the singular form, never the plural, got: {html}"
        );
    }

    #[test]
    fn the_week_screen_carries_no_masthead_and_no_way_back_to_aujourdhui() {
        let html = render(RootAtWeekScreen);

        let screen_content = &html[..html
            .find(r#"<nav class="bottom-nav""#)
            .expect("the bar renders on Week")];
        assert!(
            !screen_content.contains("masthead"),
            "the bottom bar is the navigation — the screen carries no masthead, got: {screen_content}"
        );
        assert!(
            !screen_content.contains("Aujourd&#39;hui"),
            "the back-link to Aujourd'hui left the screen for the bar, got: {screen_content}"
        );
        assert!(
            html.contains(r#"aria-label="Aujourd&#39;hui · navigation""#),
            "the bar still reaches Aujourd'hui, got: {html}"
        );
    }

    // Regression (issue #30): HabitProgress deliberately carries no `id`
    // (adr-0006), and title is NOT a unique key over the set this screen
    // renders -- an anchored habit is exempt from the title-uniqueness check
    // (add_habit.rs filters Anchored out before comparing), so two rows can
    // legally share a title. Keying `.week-habit` on `habit.title` would
    // give dioxus-core duplicate keys among siblings, which its own
    // `diff_keyed_children` rejects with `debug_assert_eq!` the moment the
    // screen re-renders. The assertion compiles out in release, so the
    // failure mode there is silent row mis-association -- exactly what a
    // key exists to prevent.
    //
    // Mounting alone never diffs (no "old" tree to compare against), so the
    // defect is invisible on first render; `mark_all_dirty` + a second
    // `render_immediate` forces the keyed-siblings diff a live re-render
    // would take. Not observable via `dioxus_ssr::render` (SSR never reads
    // `key`), so this asserts on "does not panic", not on rendered text --
    // dioxus-core's own debug assertion is the oracle here.
    //
    // The fixture spells the collision at its own site rather than
    // inheriting it from `a_habit` (retry-2: an unrelated edit to `a_habit`
    // must not silently defuse this), and the precondition assertion below
    // makes that premise fail loudly, not vacuously, if it ever stops
    // holding.
    #[test]
    fn re_rendering_the_week_screen_does_not_panic_on_same_titled_habits() {
        let mut vdom = VirtualDom::new(RootAtWeekScreenWithTwoRowsSharingATitle);
        vdom.rebuild_in_place();

        let html = dioxus_ssr::render(&vdom);
        assert!(
            html.matches(&format!(r#"week-habit-title">{SHARED_TITLE}"#))
                .count()
                >= 2,
            "this regression test only means anything while two rows share a title, got: {html}"
        );

        vdom.mark_all_dirty();
        vdom.render_immediate(&mut dioxus_core::NoOpMutations);
    }

    /// Ordered list of every `--practice-ratio: N` value found in the
    /// rendered HTML, parsed as `f64`, in document order — lets a test pin
    /// the band's normalized pebble sizes and their order, not just the
    /// pebble count.
    fn rendered_practice_ratios(html: &str) -> Vec<f64> {
        const NEEDLE: &str = "--practice-ratio: ";
        html.match_indices(NEEDLE)
            .map(|(index, _)| {
                let start = index + NEEDLE.len();
                let end = html[start..]
                    .find([';', '"'])
                    .map(|offset| start + offset)
                    .unwrap_or(html.len());
                html[start..end]
                    .parse()
                    .expect("--practice-ratio must render a valid f64")
            })
            .collect()
    }

    /// Ordered list of whether each element whose `class` starts with
    /// `prefix` also carries `is-practised`, in document order. Matches the
    /// exact class token (`prefix` alone, or `prefix` followed by a space
    /// and more classes) rather than a raw string prefix, so `"rhythm-dot"`
    /// never matches a future `"rhythm-dot-label"`.
    fn class_states(html: &str, prefix: &str) -> Vec<bool> {
        const NEEDLE: &str = "class=\"";
        let with_space = format!("{prefix} ");
        html.match_indices(NEEDLE)
            .filter_map(|(index, _)| {
                let start = index + NEEDLE.len();
                let end = start + html[start..].find('"')?;
                let class = &html[start..end];
                (class == prefix || class.starts_with(&with_space))
                    .then(|| class.contains("is-practised"))
            })
            .collect()
    }

    /// Lets a test pin the rhythm row's day order, not just how many dots
    /// are lit.
    fn rhythm_dot_states(html: &str) -> Vec<bool> {
        class_states(html, "rhythm-dot")
    }

    #[component]
    fn RootAtWeekScreenWithAGrowingHabit() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 3, TODAY - 6);
            habit.grow(LocalDate::from_epoch_day(TODAY - 4));
            habit.grow(LocalDate::from_epoch_day(TODAY - 2));
            for days_back in 0..4 {
                habit.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
            }
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: week-recap/S5
    #[test]
    fn each_habit_row_reads_its_goal_with_one_pebble_per_day_practised() {
        let html = render(RootAtWeekScreenWithAGrowingHabit);

        assert!(
            html.contains(r#"class="week-habit-goal">3 → 5 min<"#),
            "expected the row to read the starting and current goal, got: {html}"
        );
        assert_eq!(
            html.matches(r#"class="week-pebble""#).count(),
            4,
            "expected one pebble per day practised (four), not one per recorded \
             goal step (three), got: {html}"
        );
        assert_eq!(
            rendered_practice_ratios(&html),
            vec![0.8, 1.0, 1.0, 1.0],
            "expected each pebble's size normalized to the row's own maximum \
             (5) so the tallest reads 1.0, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithAGrowingHabitInEnglish() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 3, TODAY - 6);
            habit.grow(LocalDate::from_epoch_day(TODAY - 4));
            habit.grow(LocalDate::from_epoch_day(TODAY - 2));
            for days_back in 0..4 {
                habit.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
            }
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: language/S4
    #[test]
    fn a_habit_row_reads_its_starting_and_current_goal_through_the_i18n_layer() {
        let html = render(RootAtWeekScreenWithAGrowingHabitInEnglish);

        assert!(
            html.contains(r#"class="week-habit-goal">3 to 5 min<"#),
            "expected the week-habit-goal line to read the starting/current \
             goal through the i18n layer under English, not a language-neutral \
             literal, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithAFreshHabit() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            repository.save(&a_habit("h-1", 5, TODAY));
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: week-recap/S7
    #[test]
    fn a_never_practised_habits_row_draws_no_curve_at_all() {
        let html = render(RootAtWeekScreenWithAFreshHabit);

        assert!(
            html.contains(r#"class="week-habit-goal">5 min<"#),
            "expected a goal that has not moved to read without an arrow, got: {html}"
        );
        assert!(
            !html.contains("week-curve") && !html.contains("week-pebble"),
            "no day has been practised yet — the band container itself \
             must not render, not even empty, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithAFreshHabitAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            repository.save(&a_habit("h-1", 5, TODAY));
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_week_screen_in_english() {
        let html = render(RootAtWeekScreenWithAFreshHabitAndEnglishLocale);

        assert!(
            html.contains(r#"<h1 class="week-heading">This week</h1>"#),
            "expected the week heading in English, got: {html}"
        );
        assert!(
            html.contains(
                r#"<span class="week-figure">0</span><span class="week-figure-label">minutes practised</span>"#
            ),
            "expected the accumulated-minutes figure and its legend in English, got: {html}"
        );
        assert!(
            html.contains(r#"class="week-word">A perfect start. Everything is still ahead.<"#),
            "expected the fresh-start message in English, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Your rhythm over the last seven days""#),
            "expected the rhythm aria-label in English, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithALightenedHabit() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 6);
            habit.toggle_done(LocalDate::from_epoch_day(TODAY - 5));
            habit.grow(LocalDate::from_epoch_day(TODAY - 4));
            habit.toggle_done(LocalDate::from_epoch_day(TODAY - 4));
            habit.lighten(LocalDate::from_epoch_day(TODAY - 2));
            habit.toggle_done(LocalDate::from_epoch_day(TODAY - 2));
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // Unanchored: no scenario in week-recap.feature names a lightened row —
    // S5 only covers growing. `LightenGoal` is a delivered, wired use case
    // (issue #13), so a history whose maximum practised goal sits mid-row,
    // not last, is reachable today; this test pins the boundary the
    // normalization rule (`practice_ratios`, above) must hold on it.
    #[test]
    fn a_lightened_row_still_normalizes_on_its_own_maximum_practised_goal() {
        let html = render(RootAtWeekScreenWithALightenedHabit);

        let ratios = rendered_practice_ratios(&html);
        assert!(
            ratios.iter().all(|&ratio| ratio <= 1.0),
            "no bar may exceed its container: {ratios:?}"
        );
        assert_eq!(
            ratios,
            vec![5.0 / 6.0, 1.0, 5.0 / 6.0],
            "expected the row's own maximum practised goal (6, reached \
             mid-history) to normalize every bar, not the current goal (5, \
             which the row lightened back down to), got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithARhythm() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit_a = a_habit("h-1", 5, TODAY - 6);
            habit_a.toggle_done(LocalDate::from_epoch_day(TODAY - 6));
            habit_a.toggle_done(LocalDate::from_epoch_day(TODAY - 2));
            repository.save(&habit_a);
            let mut habit_b = a_habit("h-2", 5, TODAY - 6);
            habit_b.toggle_done(LocalDate::from_epoch_day(TODAY - 4));
            repository.save(&habit_b);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: week-recap/S6
    #[test]
    fn the_rhythm_row_shows_seven_dots_lit_on_practiced_days_only() {
        let html = render(RootAtWeekScreenWithARhythm);

        assert_eq!(
            rhythm_dot_states(&html),
            vec![true, false, true, false, true, false, false],
            "expected seven dots, oldest first, lit only on days at least \
             one habit was practised, faint on the rest — never a gap, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithAPractisedAndAnUnpractisedHabit() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut practised = a_habit("h-1", 5, TODAY - 6);
            practised.toggle_done(LocalDate::from_epoch_day(TODAY));
            repository.save(&practised);
            repository.save(&a_habit("h-2", 5, TODAY));
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    /// The exact `<div class="week-curve">...</div>` markup of the first
    /// curve rendered — lets a test assert byte-for-byte what a row's band
    /// carries, not just whether one class token is present. `.week-curve`
    /// never carries a second class or variant now that a band either draws
    /// pebbles or does not render at all, so a single needle is enough.
    fn the_week_curve_html(html: &str) -> &str {
        const CLOSE: &str = "</div>";
        let start = html
            .match_indices(r#"<div class="week-curve""#)
            .next()
            .map(|(index, _)| index)
            .expect("no .week-curve rendered");
        let end = html[start..]
            .find(CLOSE)
            .map(|offset| start + offset + CLOSE.len())
            .expect(".week-curve div must close");
        &html[start..end]
    }

    // @scenario: week-recap/S8
    #[test]
    fn only_the_practised_habits_row_draws_a_curve() {
        let html = render(RootAtWeekScreenWithAPractisedAndAnUnpractisedHabit);

        assert_eq!(
            html.matches(r#"class="week-curve""#).count(),
            1,
            "expected exactly one .week-curve — the practised habit's — got: {html}"
        );
        assert_eq!(
            the_week_curve_html(&html),
            r#"<div class="week-curve" aria-label="Trajectoire de Lire une page, de 5 à 5 minutes, 1 jour pratiqué"><span class="week-pebble" style="--practice-ratio: 1"></span></div>"#,
            "the practised row must gain nothing beyond its pebbles and its \
             enriched aria-label; the unpractised row must draw no band at \
             all — no counter, no mark of absence"
        );
    }

    #[component]
    fn RootAtWeekScreenWithHabitsAtTheWindowEdge() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut inside_window = a_habit("h-1", 5, TODAY - 20);
            inside_window.toggle_done(LocalDate::from_epoch_day(TODAY - 6));
            repository.save(&inside_window);
            let mut outside_window = a_habit("h-2", 5, TODAY - 20);
            outside_window.toggle_done(LocalDate::from_epoch_day(TODAY - 8));
            repository.save(&outside_window);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: week-recap/S9
    #[test]
    fn the_mini_curve_reads_the_same_rolling_window_as_the_rhythm() {
        let html = render(RootAtWeekScreenWithHabitsAtTheWindowEdge);

        assert_eq!(
            html.matches(r#"class="week-curve""#).count(),
            1,
            "a last practice six days back is still inside the rolling \
             window and must draw a bar; eight days back is already \
             outside it and must draw none, got: {html}"
        );
        assert_eq!(
            rendered_practice_ratios(&html),
            vec![1.0],
            "the one habit inside the window must draw exactly one bar, \
             got: {html}"
        );
    }

    // @algo: fluent-rs resolves a Fluent select expression by trying a
    // NumberLiteral variant key (fr.ftl's `[0]`) before computing the CLDR
    // plural category — undocumented in this crate's code.
    #[component]
    fn WeekMinutesAndLegendAtZeroOneTwoFr() -> Element {
        crate::i18n::use_locale_for_tests();
        rsx! {
            p {
                span { class: "week-figure", "0" }
                span { class: "week-figure-label", {tr!("week-minutes-practised", count: 0i64)} }
            }
            p {
                span { class: "week-figure", "1" }
                span { class: "week-figure-label", {tr!("week-minutes-practised", count: 1i64)} }
            }
            p {
                span { class: "week-figure", "2" }
                span { class: "week-figure-label", {tr!("week-minutes-practised", count: 2i64)} }
            }
        }
    }

    #[component]
    fn WeekMinutesAndLegendAtZeroOneTwoEn() -> Element {
        use_locale_for_tests_as(langid!("en"));
        rsx! {
            p {
                span { class: "week-figure", "0" }
                span { class: "week-figure-label", {tr!("week-minutes-practised", count: 0i64)} }
            }
            p {
                span { class: "week-figure", "1" }
                span { class: "week-figure-label", {tr!("week-minutes-practised", count: 1i64)} }
            }
            p {
                span { class: "week-figure", "2" }
                span { class: "week-figure-label", {tr!("week-minutes-practised", count: 2i64)} }
            }
        }
    }

    #[test]
    fn the_week_figure_reads_the_right_plural_form_at_zero_one_and_two_in_french() {
        let html = render(WeekMinutesAndLegendAtZeroOneTwoFr);

        assert!(
            html.contains(
                r#"<p><span class="week-figure">0</span><span class="week-figure-label">minutes de pratique accumulées</span></p>"#
            ),
            "expected n=0 to keep the figure and read the plural — the D1 \
             divergence from recap-minutes-label, got: {html}"
        );
        assert!(
            html.contains(
                r#"<p><span class="week-figure">1</span><span class="week-figure-label">minute de pratique accumulée</span></p>"#
            ),
            "expected n=1 to read the singular, got: {html}"
        );
        assert!(
            html.contains(
                r#"<p><span class="week-figure">2</span><span class="week-figure-label">minutes de pratique accumulées</span></p>"#
            ),
            "expected n=2 to read the plural, got: {html}"
        );
    }

    #[test]
    fn the_week_figure_reads_the_right_plural_form_at_zero_one_and_two_in_english() {
        let html = render(WeekMinutesAndLegendAtZeroOneTwoEn);

        assert!(
            html.contains(
                r#"<p><span class="week-figure">0</span><span class="week-figure-label">minutes practised</span></p>"#
            ),
            "expected n=0 to read the plural (English `other` covers 0), got: {html}"
        );
        assert!(
            html.contains(
                r#"<p><span class="week-figure">1</span><span class="week-figure-label">minute practised</span></p>"#
            ),
            "expected n=1 to read the singular, got: {html}"
        );
        assert!(
            html.contains(
                r#"<p><span class="week-figure">2</span><span class="week-figure-label">minutes practised</span></p>"#
            ),
            "expected n=2 to read the plural, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithOnePractisedDay() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 6);
            habit.toggle_done(LocalDate::from_epoch_day(TODAY));
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtWeekScreenWithTwoPractisedDays() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 6);
            habit.toggle_done(LocalDate::from_epoch_day(TODAY));
            habit.toggle_done(LocalDate::from_epoch_day(TODAY - 1));
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: language/S4
    #[test]
    fn the_curves_aria_states_the_number_of_days_practised_in_french() {
        let one = render(RootAtWeekScreenWithOnePractisedDay);
        assert!(
            one.contains("1 jour pratiqué"),
            "expected the singular count in the curve's aria-label, got: {one}"
        );

        let two = render(RootAtWeekScreenWithTwoPractisedDays);
        assert!(
            two.contains("2 jours pratiqués"),
            "expected the plural count in the curve's aria-label, got: {two}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithOnePractisedDayInEnglish() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 6);
            habit.toggle_done(LocalDate::from_epoch_day(TODAY));
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtWeekScreenWithTwoPractisedDaysInEnglish() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 6);
            habit.toggle_done(LocalDate::from_epoch_day(TODAY));
            habit.toggle_done(LocalDate::from_epoch_day(TODAY - 1));
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: language/S4
    #[test]
    fn the_curves_aria_states_the_number_of_days_practised_in_english() {
        let one = render(RootAtWeekScreenWithOnePractisedDayInEnglish);
        assert!(
            one.contains("1 day practised"),
            "expected the singular count in the curve's aria-label, got: {one}"
        );

        let two = render(RootAtWeekScreenWithTwoPractisedDaysInEnglish);
        assert!(
            two.contains("2 days practised"),
            "expected the plural count in the curve's aria-label, got: {two}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithAFullWeek() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 6);
            for days_back in 0..7 {
                habit.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
            }
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtWeekScreenWithPracticeOnOddDays() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 6);
            for days_back in [6, 4, 2] {
                habit.toggle_done(LocalDate::from_epoch_day(TODAY - days_back));
            }
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    /// The `<svg class="week-spark" …>` opening tag of the tinted card.
    fn the_staircase_tag(html: &str) -> &str {
        let start = html
            .find(r#"<svg class="week-spark""#)
            .expect("the staircase renders");
        let end = start + html[start..].find('>').expect("the tag closes");
        &html[start..end]
    }

    /// The `points` attribute of the staircase, parsed into coordinates.
    fn the_staircase_points(html: &str) -> Vec<(f64, f64)> {
        const NEEDLE: &str = r#"points=""#;
        let start = html.find(NEEDLE).expect("the staircase renders") + NEEDLE.len();
        let end = start
            + html[start..]
                .find('"')
                .expect("the points attribute closes");
        html[start..end]
            .split(' ')
            .map(|pair| {
                let (x, y) = pair.split_once(',').expect("a point reads x,y");
                (
                    x.parse().expect("x parses as f64"),
                    y.parse().expect("y parses as f64"),
                )
            })
            .collect()
    }

    fn the_staircases_last_point(html: &str) -> (f64, f64) {
        *the_staircase_points(html)
            .last()
            .expect("the trace has an end")
    }

    /// Counts the rising (`dx == 0`) and flat (`dy == 0`) segments of a trace.
    fn staircase_segments(points: &[(f64, f64)]) -> (usize, usize) {
        points.windows(2).fold((0, 0), |(rises, flats), pair| {
            let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
            (rises + usize::from(x0 == x1), flats + usize::from(y0 == y1))
        })
    }

    /// The value of a numeric attribute, read from the staircase onwards so
    /// the bottom bar's own `<circle cx>` can never satisfy it first.
    fn a_staircase_attribute(html: &str, name: &str) -> f64 {
        let from_staircase = &html[html
            .find(r#"<svg class="week-spark""#)
            .expect("the staircase renders")..];
        let needle = format!(r#"{name}=""#);
        let start = from_staircase.find(&needle).expect("the attribute renders") + needle.len();
        let end = start
            + from_staircase[start..]
                .find('"')
                .expect("the attribute closes");
        from_staircase[start..end]
            .parse()
            .expect("the attribute parses as f64")
    }

    // @scenario: week-recap/S10
    #[test]
    fn the_staircase_holds_flat_when_the_week_holds_no_practice() {
        let html = render(RootAtRestingWeekScreen);
        let points = the_staircase_points(&html);

        let (rises, flats) = staircase_segments(&points);
        assert_eq!(
            (rises, flats),
            (0, 7),
            "seven days without practice draw seven flat runs and no step, \
             got: {html}"
        );
        assert!(
            points.windows(2).all(|pair| pair[0].1 == pair[1].1),
            "the whole trace stays at one level, got: {html}"
        );
    }

    // @scenario: week-recap/S10
    #[test]
    fn the_staircase_rises_once_per_day_practised() {
        let html = render(RootAtWeekScreenWithAFullWeek);
        let points = the_staircase_points(&html);

        let (rises, flats) = staircase_segments(&points);
        assert_eq!(
            (rises, flats),
            (7, 7),
            "seven practised days step up once each, got: {html}"
        );
        assert!(
            points.windows(2).all(|pair| pair[0].1 >= pair[1].1),
            "the trace never falls, got: {html}"
        );
        assert_eq!(
            the_staircases_last_point(&html).1,
            5.0,
            "seven practised days reach the top of the box, got: {html}"
        );
    }

    // @scenario: week-recap/S10
    #[test]
    fn the_staircase_holds_its_level_across_a_rest_day() {
        let html = render(RootAtWeekScreenWithPracticeOnOddDays);
        let points = the_staircase_points(&html);

        assert_eq!(
            staircase_segments(&points),
            (3, 7),
            "practice on the window's days 1, 3 and 5 steps up three times, and \
             every rest day between two practised ones holds a flat run, got: {html}"
        );
        assert!(
            points.windows(2).all(|pair| pair[0].1 >= pair[1].1),
            "a day of rest is a plateau, never a fall, got: {html}"
        );
        assert_eq!(
            the_staircases_last_point(&html).0,
            110.0,
            "the trace ends on the last day of the window, got: {html}"
        );
    }

    // @scenario: week-recap/S10
    #[test]
    fn the_staircase_stays_out_of_the_accessibility_tree() {
        let html = render(RootAtWeekScreen);

        assert!(
            the_staircase_tag(&html).contains(r#"aria-hidden="true""#),
            "the rhythm row already says the seven days in dots — the \
             staircase must stay out of the accessibility tree, got: {html}"
        );
    }

    // @scenario: week-recap/S10
    #[test]
    fn the_staircase_ends_on_its_last_traced_point() {
        let html = render(RootAtWeekScreen);
        let (x, y) = the_staircases_last_point(&html);

        assert_eq!(
            (
                a_staircase_attribute(&html, "cx"),
                a_staircase_attribute(&html, "cy")
            ),
            (x, y),
            "the end pebble sits on the last point of the trace, got: {html}"
        );
    }
}
