use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;
use kayzen_core::habit_management::queries::get_week_recap::WeekMessage;

#[component]
pub fn Week() -> Element {
    let services = use_context::<Services>();
    let recap = use_signal(move || services.get_week_recap.handle());
    let recap = recap();
    let minutes = recap.minutes_practised;
    let figure = if minutes == 1 {
        format!("{minutes} minute de pratique accumulée")
    } else {
        format!("{minutes} minutes de pratique accumulées")
    };

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                Link { class: "quiet-link", to: Route::Today {}, "← Aujourd'hui" }
            }
            h1 { class: "greeting", "Cette semaine" }
            p { class: "week-figure", "{figure}" }
            p { class: "week-word", "{week_copy(recap.message)}" }

            div { class: "rhythm", "aria-label": "Votre rythme sur les sept derniers jours",
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
                        p { class: "week-habit-title", "{habit.title}" }
                        p { class: "week-habit-journey", "{habit.starting_goal} → {habit.current_goal} min" }
                        div {
                            class: "week-curve",
                            "aria-label": "Trajectoire de {habit.title}, de {habit.starting_goal} à {habit.current_goal} minutes",
                            for (step_offset, ratio) in step_ratios(&habit.steps).into_iter().enumerate()
                            {
                                span { key: "{step_offset}", class: "step-bar", style: "--step-ratio: {ratio}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Each bar's height relative to its own row's tallest step (adr-0010: core
/// returns numbers, the view decides how to draw them) — never an absolute
/// minute value. The owner's call: a 2→3 habit and a 30→32 habit draw the
/// same shape, because the row shows relative progression, not absolute
/// effort. `unwrap_or(1)` only guards an empty slice; `steps` is never empty
/// in practice (`HabitProgress::for_habit` always seeds at least one step),
/// so it never actually influences a returned ratio.
#[must_use]
fn step_ratios(steps: &[u32]) -> Vec<f64> {
    let row_max = steps.iter().copied().max().unwrap_or(1) as f64;
    steps.iter().map(|&step| step as f64 / row_max).collect()
}

#[must_use]
fn week_copy(message: WeekMessage) -> &'static str {
    match message {
        WeekMessage::FreshStart => "Un début parfait. Tout est encore devant.",
        WeekMessage::Resting => "Cette semaine se repose. Elle vous attend, sans presser.",
        WeekMessage::Growing => "Vous avancez, à votre rythme.",
    }
}

#[cfg(test)]
mod tests {
    use crate::composition::Services;
    use crate::route::Route;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use dioxus::prelude::*;
    use kayzen_core::habit_management::domain::goal::Goal;
    use kayzen_core::habit_management::domain::habit::Habit;
    use kayzen_core::habit_management::domain::habit_id::HabitId;
    use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
    use kayzen_core::habit_management::domain::habit_title::HabitTitle;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
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
    // - the figure states minutes practised across every habit (S1).
    // - paused and anchored habits still count toward the figure (S2, sum half
    //   only — see get_week_recap.rs's test of the same name for the split).
    // - a fresh week reads its gentle word (S3).
    // - a week without recent practice reads rest, without blame (S4).
    // - the masthead back-link returns to Aujourd'hui.
    // - each habit's row reads its journey, one bar per goal step, not one
    //   per completed day (S5), each bar's height normalized to that row's
    //   own maximum step, never an absolute minute value (owner decision,
    //   2026-08-21: relative progression, not absolute effort).
    // - a brand-new, not-yet-practised habit's row still reads a journey (S7).
    // - a row lightened back down (its maximum step sits mid-history, not
    //   last) still normalizes on that row's own maximum, never on its
    //   current goal — no bar may exceed its container.
    // - the rhythm row shows seven dots, oldest first, lit on practiced days
    //   and faint on the rest, never a gap (S6).
    // - a habit practised in the rolling window draws its mini-curve's bars
    //   with is-practised; a habit not practised in the window draws its
    //   bars without it, and gets nothing else added (S8).

    // @scenario: week-recap/S1
    #[test]
    fn the_week_screen_states_the_accumulated_minutes() {
        let html = render(RootAtWeekScreen);

        assert!(
            html.contains("30 minutes de pratique accumulées"),
            "expected the large figure to name accumulated practice, never \
             gain over the starting goal, got: {html}"
        );
        assert!(
            html.contains("Vous avancez"),
            "recent practice must read the Growing word, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithPausedAndAnchoredHabits() -> Element {
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
            html.contains("35 minutes de pratique accumulées"),
            "pausing or anchoring must never take lived minutes back, got: {html}"
        );
    }

    #[component]
    fn RootAtFreshWeekScreen() -> Element {
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
            html.contains("0 minutes de pratique accumulées"),
            "a fresh week's figure is never a bare zero — it always carries \
             its label, got: {html}"
        );
    }

    #[component]
    fn RootAtRestingWeekScreen() -> Element {
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
            html.contains("1 minute de pratique accumulée"),
            "one minute must read the singular form, never the plural, got: {html}"
        );
    }

    #[test]
    fn the_masthead_back_link_returns_to_aujourdhui() {
        let html = render(RootAtWeekScreen);

        assert!(
            html.contains(r#"href="/""#) && html.contains("Aujourd&#39;hui"),
            "expected the masthead back-link idiom to Aujourd'hui, got: {html}"
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

    /// Ordered list of every `--step-ratio: N` value found in the rendered
    /// HTML, parsed as `f64`, in document order — lets a test pin the
    /// mini-curve's normalized bar heights and their order, not just the
    /// bar count.
    fn step_bar_ratios(html: &str) -> Vec<f64> {
        const NEEDLE: &str = "--step-ratio: ";
        html.match_indices(NEEDLE)
            .map(|(index, _)| {
                let start = index + NEEDLE.len();
                let end = html[start..]
                    .find([';', '"'])
                    .map(|offset| start + offset)
                    .unwrap_or(html.len());
                html[start..end]
                    .parse()
                    .expect("--step-ratio must render a valid f64")
            })
            .collect()
    }

    /// Ordered list of whether each `.rhythm-dot` carries `is-practised`, in
    /// document order — lets a test pin the rhythm row's day order, not just
    /// how many dots are lit.
    fn rhythm_dot_states(html: &str) -> Vec<bool> {
        const NEEDLE: &str = "class=\"";
        html.match_indices(NEEDLE)
            .filter_map(|(index, _)| {
                let start = index + NEEDLE.len();
                let end = start + html[start..].find('"')?;
                let class = &html[start..end];
                class
                    .starts_with("rhythm-dot")
                    .then(|| class.contains("is-practised"))
            })
            .collect()
    }

    #[component]
    fn RootAtWeekScreenWithAGrowingHabit() -> Element {
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
    fn each_habit_row_reads_its_journey_with_one_bar_per_goal_step() {
        let html = render(RootAtWeekScreenWithAGrowingHabit);

        assert!(
            html.contains("3 → 5 min"),
            "expected the row to read the starting and current goal, got: {html}"
        );
        assert_eq!(
            step_bar_ratios(&html),
            vec![0.6, 0.8, 1.0],
            "expected one bar per goal step (three steps were recorded), not \
             one per completed day (four), each normalized to the row's own \
             maximum (5) so the tallest bar reads 1.0, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithAFreshHabit() -> Element {
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
    fn a_brand_new_habit_already_shows_its_journey_on_the_week_screen() {
        let html = render(RootAtWeekScreenWithAFreshHabit);

        assert!(
            html.contains("5 → 5 min"),
            "expected an empty start to still read as a start, got: {html}"
        );
        assert_eq!(
            step_bar_ratios(&html),
            vec![1.0],
            "expected a single bar for a not-yet-grown habit, drawn at its \
             row's own maximum, got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithALightenedHabit() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/week")));
        });
        use_context_provider(|| {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            let mut habit = a_habit("h-1", 5, TODAY - 6);
            habit.grow(LocalDate::from_epoch_day(TODAY - 4));
            habit.lighten(LocalDate::from_epoch_day(TODAY - 2));
            repository.save(&habit);
            services_with(repository)
        });
        rsx! {
            Router::<Route> {}
        }
    }

    // Unanchored: no scenario in week-recap.feature names a lightened row —
    // S5 only covers growing. `LightenGoal` is a delivered, wired use case
    // (issue #13), so a history whose maximum step sits mid-row, not last,
    // is reachable today; this test pins the boundary the normalization
    // rule (`step_ratios`, above) must hold on it.
    #[test]
    fn a_lightened_row_still_normalizes_on_its_own_maximum_step() {
        let html = render(RootAtWeekScreenWithALightenedHabit);

        let ratios = step_bar_ratios(&html);
        assert!(
            ratios.iter().all(|&ratio| ratio <= 1.0),
            "no bar may exceed its container: {ratios:?}"
        );
        assert_eq!(
            ratios,
            vec![5.0 / 6.0, 1.0, 5.0 / 6.0],
            "expected the row's own maximum step (6, reached mid-history) to \
             normalize every bar, not the current goal (5, which the row \
             lightened back down to), got: {html}"
        );
    }

    #[component]
    fn RootAtWeekScreenWithARhythm() -> Element {
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

    /// Ordered list of whether each `.step-bar` carries `is-practised`, in
    /// document order — mirrors `rhythm_dot_states`, reading the class
    /// attribute rather than counting bars.
    fn step_bar_practised_states(html: &str) -> Vec<bool> {
        const NEEDLE: &str = "class=\"";
        html.match_indices(NEEDLE)
            .filter_map(|(index, _)| {
                let start = index + NEEDLE.len();
                let end = start + html[start..].find('"')?;
                let class = &html[start..end];
                class
                    .starts_with("step-bar")
                    .then(|| class.contains("is-practised"))
            })
            .collect()
    }

    // @scenario: week-recap/S8
    #[test]
    fn a_practised_rows_curve_reads_in_the_accent_an_unpractised_rows_does_not() {
        let html = render(RootAtWeekScreenWithAPractisedAndAnUnpractisedHabit);

        assert_eq!(
            step_bar_practised_states(&html),
            vec![true, false],
            "expected the practised habit's row to draw its bar with \
             is-practised and the unpractised habit's row to draw its bar \
             without it, got: {html}"
        );
    }
}
