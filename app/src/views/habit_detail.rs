use crate::composition::Services;
use crate::i18n::tr;
use crate::route::Route;
use dioxus::prelude::*;
use kayzen_core::habit_management::queries::get_habit_detail::HabitDetail as HabitDetailData;
use kayzen_core::habit_management::queries::get_habit_detail::HabitState;
use kayzen_core::habit_management::queries::get_habit_detail::PracticeDay;

#[component]
pub fn HabitDetail(id: String) -> Element {
    let services = use_context::<Services>();
    let mut detail = use_signal({
        let services = services.clone();
        let id = id.clone();
        move || services.get_habit_detail.handle(&id)
    });

    match detail() {
        Some(habit) => {
            let back_link = rsx! {
                Link {
                    class: "detail-back",
                    to: Route::Today {},
                    aria_label: tr!("detail-back-to-today"),
                    svg {
                        class: "detail-back-icon",
                        view_box: "0 0 24 24",
                        "aria-hidden": "true",
                        "focusable": "false",
                        path { d: "M15 5l-7 7 7 7" }
                    }
                }
            };

            let week = rsx! {
                div {
                    class: "week-card",
                    "aria-label": tr!("staircase-aria", goal: habit.current_goal as i64),
                    div { class: "pebble-track",
                        for (day_offset, (day, ratio)) in
                            habit.days.iter().zip(day_ratios(&habit.days)).enumerate()
                        {
                            span {
                                key: "{day_offset}",
                                class: day_pebble_class(day.done, day_offset + 1 == habit.days.len()),
                                style: "--day-ratio: {ratio}",
                            }
                        }
                    }
                }
            };

            match habit.state {
                HabitState::Active => rsx! {
                    div { class: "screen detail",
                        {back_link}
                        header { class: "detail-head",
                            h1 { class: "greeting", "{habit.title}" }
                            span { class: "dose", {tr!("habit-detail-active-dose", goal: habit.current_goal as i64)} }
                        }

                        {week}

                        div { class: "action-dock",
                            Link {
                                class: "btn btn-primary btn-block action-primary",
                                to: Route::Ritual { id: habit.id.clone() },
                                svg {
                                    class: "action-glyph",
                                    view_box: "0 0 24 24",
                                    "aria-hidden": "true",
                                    "focusable": "false",
                                    path { d: "M7 5l12 7-12 7z" }
                                }
                                {tr!("start-ritual-label")}
                            }
                            button {
                                class: if habit.done_today { "btn btn-secondary action-done is-done" } else { "btn btn-secondary action-done" },
                                aria_label: if habit.done_today { tr!("habit-detail-done-aria", title: habit.title.clone()) } else { tr!("habit-detail-mark-done-aria", title: habit.title.clone()) },
                                onclick: {
                                    let services = services.clone();
                                    let id = id.clone();
                                    move |_| detail.set(mark_done_and_reload(&services, &id))
                                },
                                if habit.done_today {
                                    svg {
                                        class: "action-done-check",
                                        view_box: "0 0 24 24",
                                        "aria-hidden": "true",
                                        "focusable": "false",
                                        path { d: "M5 12l5 5L19 7" }
                                    }
                                }
                                {tr!("habit-detail-mark-done-label")}
                            }
                        }

                        div { class: "pace",
                            h2 { class: "pace-heading", {tr!("adjust-goal-eyebrow")} }
                            div { class: "pace-grid",
                                button {
                                    class: "pace-tile is-grow",
                                    aria_label: tr!("grow-goal-aria", goal: habit.next_goal_up as i64, title: habit.title.clone()),
                                    onclick: {
                                        let services = services.clone();
                                        let id = id.clone();
                                        move |_| detail.set(grow_and_reload(&services, &id))
                                    },
                                    span { class: "pace-glyph", "aria-hidden": "true", "+" }
                                    span { class: "pace-label", {tr!("grow-goal-label", goal: habit.next_goal_up as i64)} }
                                }
                                button {
                                    class: "pace-tile is-lighten",
                                    aria_label: tr!("lighten-goal-aria", goal: habit.next_goal_down as i64, title: habit.title.clone()),
                                    onclick: {
                                        let services = services.clone();
                                        let id = id.clone();
                                        move |_| detail.set(lighten_and_reload(&services, &id))
                                    },
                                    span { class: "pace-glyph", "aria-hidden": "true", "−" }
                                    span { class: "pace-label", {tr!("lighten-goal-label", goal: habit.next_goal_down as i64)} }
                                }
                            }
                        }

                        div { class: "lifecycle",
                            button {
                                class: "lifecycle-gesture",
                                aria_label: tr!("pause-habit-aria", title: habit.title.clone()),
                                onclick: {
                                    let services = services.clone();
                                    let id = id.clone();
                                    move |_| detail.set(pause_and_reload(&services, &id))
                                },
                                svg {
                                    class: "lifecycle-icon",
                                    view_box: "0 0 24 24",
                                    "aria-hidden": "true",
                                    "focusable": "false",
                                    path { d: "M9 6v12M15 6v12" }
                                }
                                {tr!("pause-habit-label")}
                            }
                            button {
                                class: "lifecycle-gesture is-anchor",
                                aria_label: tr!("anchor-habit-aria", title: habit.title.clone()),
                                onclick: {
                                    let services = services.clone();
                                    let id = id.clone();
                                    move |_| detail.set(anchor_and_reload(&services, &id))
                                },
                                svg {
                                    class: "lifecycle-icon",
                                    view_box: "0 0 24 24",
                                    "aria-hidden": "true",
                                    "focusable": "false",
                                    ellipse { cx: "12", cy: "15", rx: "7", ry: "4.5" }
                                    path { d: "M12 10.5V4" }
                                }
                                {tr!("anchor-habit-label")}
                            }
                        }
                    }
                },
                HabitState::Paused => rsx! {
                    div { class: "screen detail",
                        {back_link}
                        header { class: "detail-head",
                            h1 { class: "greeting", "{habit.title}" }
                            span { class: "dose", {tr!("habit-detail-paused-dose", goal: habit.current_goal as i64)} }
                        }

                        {week}

                        div { class: "action-dock",
                            button {
                                class: "btn btn-primary btn-block",
                                aria_label: tr!("resume-habit-aria", title: habit.title.clone()),
                                onclick: {
                                    let services = services.clone();
                                    let id = id.clone();
                                    move |_| detail.set(resume_and_reload(&services, &id))
                                },
                                {tr!("resume-habit-label")}
                            }
                        }
                    }
                },
                HabitState::Anchored => rsx! {
                    div { class: "screen detail",
                        {back_link}
                        header { class: "detail-head",
                            h1 { class: "greeting", "{habit.title}" }
                            span { class: "dose", {tr!("habit-detail-anchored-dose", goal: habit.current_goal as i64)} }
                        }

                        {week}
                    }
                },
            }
        }
        None => rsx! {
            div { class: "screen",
                p { class: "lede", {tr!("habit-not-found-message")} }
                Link { class: "quiet-link", to: Route::Today {}, {tr!("habit-not-found-back-link")} }
            }
        },
    }
}

#[must_use]
fn grow_and_reload(services: &Services, id: &str) -> Option<HabitDetailData> {
    services.grow_goal.execute(id).ok();
    services.get_habit_detail.handle(id)
}

#[must_use]
fn lighten_and_reload(services: &Services, id: &str) -> Option<HabitDetailData> {
    services.lighten_goal.execute(id).ok();
    services.get_habit_detail.handle(id)
}

#[must_use]
fn pause_and_reload(services: &Services, id: &str) -> Option<HabitDetailData> {
    services.pause_habit.execute(id).ok();
    services.get_habit_detail.handle(id)
}

#[must_use]
fn mark_done_and_reload(services: &Services, id: &str) -> Option<HabitDetailData> {
    services.mark_done.execute(id).ok();
    services.get_habit_detail.handle(id)
}

#[must_use]
fn resume_and_reload(services: &Services, id: &str) -> Option<HabitDetailData> {
    services.resume_habit.execute(id).ok();
    services.get_habit_detail.handle(id)
}

#[must_use]
fn anchor_and_reload(services: &Services, id: &str) -> Option<HabitDetailData> {
    services.anchor_habit.execute(id).ok();
    services.get_habit_detail.handle(id)
}

/// One day's pebble class: filled in accent when the day was practised, in
/// plain outline when it was not, and in dashed outline for today while
/// nothing has been done — the shape cue that tells today apart without
/// leaning on colour alone (06-style-galets rule #32). `days` always ends on
/// today (get_habit_detail's window), so the caller reads it off the offset.
#[must_use]
fn day_pebble_class(done: bool, is_today: bool) -> &'static str {
    match (done, is_today) {
        (true, _) => "day-pebble is-done",
        (false, true) => "day-pebble is-today",
        (false, false) => "day-pebble",
    }
}

/// Each day's pebble size relative to its own window's tallest goal
/// (adr-0010: core returns numbers, the view decides how to draw them) —
/// never an absolute minute value. Same normalization as the one the Week
/// screen uses (owner ruling, 2026-08-21): the week draws one habit's
/// own trajectory, so there is no cross-habit comparison to lose.
/// `unwrap_or(1)` only guards an
/// empty slice; `days` always holds `WINDOW_DAYS` entries in practice, so it
/// never actually influences a returned ratio.
#[must_use]
fn day_ratios(days: &[PracticeDay]) -> Vec<f64> {
    let window_max = days.iter().map(|day| day.goal).max().unwrap_or(1) as f64;
    days.iter()
        .map(|day| day.goal as f64 / window_max)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::use_locale_for_tests_as;
    use crate::views::click_harness::Screen;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use dioxus_i18n::unic_langid::langid;
    use kayzen_core::habit_management::domain::goal::Goal;
    use kayzen_core::habit_management::domain::habit::Habit;
    use kayzen_core::habit_management::domain::habit_id::HabitId;
    use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
    use kayzen_core::habit_management::domain::habit_title::HabitTitle;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use kayzen_core::habit_management::queries::get_habit_detail::HabitState;
    use kayzen_core::shared::clock::Clock;
    use kayzen_core::shared::local_date::LocalDate;
    use std::rc::Rc;

    struct FixedClock(LocalDate);

    impl Clock for FixedClock {
        fn today(&self) -> LocalDate {
            self.0
        }
    }

    fn a_habit() -> Habit {
        Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        )
    }

    fn services_with_one_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        Services::with_repository(repository)
    }

    fn services_with_no_habits() -> Services {
        Services::with_repository(Rc::new(InMemoryHabitRepository::new()))
    }

    // A habit at the floor, done today: today's practice is recorded, so every
    // pebble is filled and no day carries the dashed "today" outline.
    fn services_with_a_floor_habit_done_today() -> Services {
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_005)));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(1).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        habit.toggle_done(clock.today());
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    // S4's Given, verbatim: a habit completed on 10 of the last 14 days — the
    // exact shape a stability detector would key on (adr-0008 deleted that
    // detector; this fixture exists to keep it deleted).
    fn services_with_a_habit_done_ten_of_the_last_fourteen_days() -> Services {
        let today = LocalDate::from_epoch_day(20_020);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        for days_back in 0..10 {
            habit.toggle_done(today.minus_days(days_back));
        }
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtHabitDoneTenOfLastFourteenDays() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_done_ten_of_the_last_fourteen_days);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtKnownHabit() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtKnownHabitAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_habit_detail_screen_in_english() {
        let html = render(RootAtKnownHabitAndEnglishLocale);

        assert!(
            html.contains(r#"class="dose">every day · 5 min<"#),
            "expected the active-dose line in English, got: {html}"
        );
        assert!(
            html.contains(r#"class="pace-heading">Adjust, at your own pace<"#),
            "expected the pace heading in English, got: {html}"
        );
        assert!(
            html.contains(">Start my practice<"),
            "expected the start-ritual gesture in English, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Pause, no guilt · Lire une page""#)
                && html.contains(">Pause, no guilt<"),
            "expected the pause gesture in English, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="It&#39;s done · Lire une page""#)
                && html.contains(">It&#39;s done<"),
            "expected the day-is-done gesture in English, got: {html}"
        );
    }

    #[component]
    fn RootAtUnknownHabit() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/missing")));
        });
        use_context_provider(services_with_no_habits);
        rsx! {
            Router::<Route> {}
        }
    }

    fn services_with_one_paused_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.pause().expect("a fresh habit is active");
        repository.save(&habit);
        Services::with_repository(repository)
    }

    fn services_with_one_anchored_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.anchor().expect("a fresh habit is active");
        repository.save(&habit);
        Services::with_repository(repository)
    }

    #[component]
    fn RootAtPausedHabit() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_paused_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtAnchoredHabit() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_anchored_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtFloorHabitDoneToday() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_floor_habit_done_today);
        rsx! {
            Router::<Route> {}
        }
    }

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    /// Ordered list of every `--day-ratio: N` value found in the rendered
    /// HTML, parsed as `f64`, in document order — lets a test pin the
    /// week's normalized pebble sizes and their order, not just the pebble
    /// count (same normalization the Week screen uses, owner ruling,
    /// 2026-08-21).
    fn day_pebble_ratios(html: &str) -> Vec<f64> {
        const NEEDLE: &str = "--day-ratio: ";
        html.match_indices(NEEDLE)
            .map(|(index, _)| {
                let start = index + NEEDLE.len();
                let end = html[start..]
                    .find([';', '"'])
                    .map(|offset| start + offset)
                    .unwrap_or(html.len());
                html[start..end]
                    .parse()
                    .expect("--day-ratio must render a valid f64")
            })
            .collect()
    }

    #[test]
    fn clicking_grow_raises_the_goal_and_re_renders_the_dose() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("Passer à 6 min · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("chaque jour · 6 min"),
            "expected the raised goal to appear after the click, got: {html}"
        );
    }

    #[test]
    fn clicking_lighten_lowers_the_goal_and_re_renders_the_dose() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("Alléger à 4 min · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("chaque jour · 4 min"),
            "expected the lowered goal to appear after the click, got: {html}"
        );
    }

    #[test]
    fn clicking_pause_re_renders_the_paused_banner() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("Mettre en pause, sans culpabilité · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("en pause · 5 min"),
            "expected the paused banner after the click, got: {html}"
        );
    }

    #[test]
    fn clicking_anchor_re_renders_the_anchored_banner() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("L'ancrer · elle est devenue naturelle · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("ancrée · 5 min"),
            "expected the anchored banner after the click, got: {html}"
        );
    }

    #[test]
    fn clicking_la_reprendre_re_renders_the_active_dose() {
        let mut screen = Screen::open(RootAtPausedHabit);

        screen.click("La reprendre · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("chaque jour · 5 min"),
            "expected the active dose to re-render after the click, got: {html}"
        );
    }

    #[test]
    fn grow_and_reload_raises_the_goal_and_returns_the_refreshed_detail() {
        let services = services_with_one_habit();

        let detail = grow_and_reload(&services, "h-1");

        assert_eq!(
            detail.map(|d| (d.current_goal, d.next_goal_up)),
            Some((6, 7)),
            "expected the gesture to have run before the screen re-reads the habit"
        );
    }

    #[test]
    fn lighten_and_reload_lowers_the_goal_and_returns_the_refreshed_detail() {
        let services = services_with_one_habit();

        let detail = lighten_and_reload(&services, "h-1");

        assert_eq!(
            detail.map(|d| (d.current_goal, d.next_goal_down)),
            Some((4, 3)),
            "expected the gesture to have run before the screen re-reads the habit"
        );
    }

    #[test]
    fn pause_and_reload_pauses_the_habit_and_returns_the_refreshed_detail() {
        let services = services_with_one_habit();

        let detail = pause_and_reload(&services, "h-1");

        assert_eq!(
            detail.map(|d| d.state),
            Some(HabitState::Paused),
            "expected the gesture to have run before the screen re-reads the habit"
        );
    }

    #[test]
    fn mark_done_and_reload_records_today_and_returns_the_refreshed_detail() {
        let services = services_with_one_habit();

        let detail = mark_done_and_reload(&services, "h-1");

        assert_eq!(
            detail.map(|d| d.done_today),
            Some(true),
            "expected the gesture to have run before the screen re-reads the habit"
        );
    }

    #[test]
    fn a_second_tap_through_the_detail_gesture_un_records_today() {
        let services = services_with_one_habit();

        let recorded = mark_done_and_reload(&services, "h-1");
        let un_recorded = mark_done_and_reload(&services, "h-1");

        assert_eq!(
            (
                recorded.map(|d| d.done_today),
                un_recorded.map(|d| d.done_today)
            ),
            (Some(true), Some(false)),
            "expected the same-day toggle to hold from the detail screen too"
        );
    }

    #[test]
    fn anchor_and_reload_anchors_the_habit_and_returns_the_refreshed_detail() {
        let services = services_with_one_habit();

        let detail = anchor_and_reload(&services, "h-1");

        assert_eq!(
            detail.map(|d| d.state),
            Some(HabitState::Anchored),
            "expected the gesture to have run before the screen re-reads the habit"
        );
    }

    #[test]
    fn resume_and_reload_resumes_the_habit_and_returns_the_refreshed_detail() {
        let services = services_with_one_paused_habit();

        let detail = resume_and_reload(&services, "h-1");

        assert_eq!(
            detail.map(|d| d.state),
            Some(HabitState::Active),
            "expected the gesture to have run before the screen re-reads the habit"
        );
    }

    #[test]
    fn a_known_habit_renders_its_title_and_goal() {
        let html = render(RootAtKnownHabit);

        assert!(
            html.contains("Lire une page"),
            "expected the habit title, got: {html}"
        );
        assert!(
            html.contains("chaque jour · 5 min"),
            "expected the dose, got: {html}"
        );
        assert!(
            html.contains("Passer à 6 min"),
            "expected the grow-goal button offering the next step up, got: {html}"
        );
        assert!(
            html.contains("Mettre en pause"),
            "expected the pause gesture to be offered on an active habit, got: {html}"
        );
        assert!(
            html.contains("L&#39;ancrer · elle est devenue naturelle"),
            "expected the anchor gesture's full copy on an active habit, got: {html}"
        );
    }

    #[test]
    fn the_ritual_gesture_states_no_duration_the_habit_does_not_have() {
        let html = render(RootAtKnownHabit);

        assert!(
            html.contains("Commencer ma pratique"),
            "expected the ritual gesture in its neutral, duration-free wording, got: {html}"
        );
        assert!(
            !html.contains("Faire ma minute"),
            "expected the old copy asserting a duration to be gone, got: {html}"
        );
    }

    // @scenario: adjust-goal/S4
    #[test]
    fn both_gestures_stay_offered_whatever_the_habits_history() {
        let html = render(RootAtFloorHabitDoneToday);

        assert!(
            html.contains("Passer à 2 min"),
            "expected the grow-goal button even at the floor, got: {html}"
        );
        assert!(
            html.contains("Alléger à 1 min"),
            "expected the lighten-goal button even at the floor, got: {html}"
        );
        assert!(
            !html.contains("disabled"),
            "expected neither gesture to carry a disabled attribute, got: {html}"
        );
    }

    // @scenario: practice-staircase/S5
    #[test]
    fn the_week_draws_one_pebble_for_each_day_inside_its_card() {
        let html = render(RootAtKnownHabit);
        assert!(
            html.contains(r#"class="week-card""#) && html.contains(r#"class="pebble-track""#),
            "expected the seven days to live in a card, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Vos sept derniers jours, objectif actuel 5 minutes""#),
            "expected the band to keep its own accessible name, got: {html}"
        );
        assert_eq!(
            html.matches("day-pebble").count(),
            7,
            "expected one pebble per calendar day of the window, got: {html}"
        );
    }

    // @scenario: practice-staircase/S2
    #[test]
    fn only_the_practised_days_are_filled_and_the_rest_stay_in_outline() {
        let html = render(RootAtFloorHabitDoneToday);

        assert_eq!(
            html.matches("day-pebble").count(),
            7,
            "expected the missed days to keep their pebble rather than leave a gap, got: {html}"
        );
        assert_eq!(
            html.matches("day-pebble is-done").count(),
            1,
            "expected only the one practised day filled, got: {html}"
        );
        assert_eq!(
            html.matches("day-pebble is-today").count(),
            0,
            "expected today to read filled like any practised day, never dashed, got: {html}"
        );
    }

    // @scenario: practice-staircase/S7
    #[test]
    fn today_is_told_apart_by_a_dashed_outline_until_it_is_practised() {
        let html = render(RootAtKnownHabit);

        assert_eq!(
            html.matches("is-today").count(),
            1,
            "expected exactly one pebble to stand for today, got: {html}"
        );
        let today = &html[html
            .rfind(r#"class="day-pebble"#)
            .expect("expected at least one pebble in the week")..];
        assert!(
            today.starts_with(r#"class="day-pebble is-today""#),
            "expected today, the last of the seven, to carry the dashed outline, got: {today}"
        );
        assert!(
            !html.contains("is-done"),
            "expected no practised day for a habit never marked done, got: {html}"
        );
    }

    #[test]
    fn the_detail_opens_on_a_round_back_gesture_named_by_its_own_key() {
        let html = render(RootAtKnownHabit);

        assert!(
            html.contains(r#"aria-label="Retour à aujourd&#39;hui""#),
            "expected the back gesture to be announced by its own key, got: {html}"
        );
        assert!(
            html.contains(r#"class="detail-back""#) && html.contains(r#"class="detail-back-icon""#),
            "expected a round back target carrying its chevron, got: {html}"
        );
        assert!(
            !html.contains("quiet-link"),
            "expected the old masthead text link to be gone, got: {html}"
        );
    }

    #[test]
    fn the_primary_action_lives_in_a_dock_after_the_week_card() {
        let html = render(RootAtKnownHabit);

        let card = html
            .find(r#"class="week-card""#)
            .expect("expected the week card");
        let dock = html
            .find(r#"class="action-dock""#)
            .expect("expected the primary action to live in its own dock");
        let pace = html
            .find(r#"class="pace-grid""#)
            .expect("expected the pace zone");
        assert!(
            card < dock && dock < pace,
            "expected the dock after the week card and before the settings, so \
             the reading order still meets the action first, got: {html}"
        );
        assert!(
            html.contains(r#"class="btn btn-primary btn-block action-primary""#)
                && html.contains(r#"class="action-glyph""#)
                && html.contains(r#"href="/habit/h-1/ritual""#)
                && html.contains(">Commencer ma pratique<"),
            "expected the dock to keep the action's destination, its label and \
             its decorative triangle, got: {html}"
        );
    }

    #[test]
    fn the_dock_offers_the_practice_gesture_beside_the_one_that_says_the_day_is_done() {
        let html = render(RootAtKnownHabit);

        let dock = &html[html
            .find(r#"class="action-dock""#)
            .expect("expected the dock")..];
        let dock_actions = &dock[..dock
            .find(r#"class="pace-grid""#)
            .expect("expected the pace zone to close the dock's block")];

        assert!(
            dock_actions.contains(r#"href="/habit/h-1/ritual""#)
                && dock_actions.contains(">Commencer ma pratique<"),
            "expected the practice gesture to stay in the dock, got: {html}"
        );
        assert!(
            dock_actions.contains(">C&#39;est fait<"),
            "expected the gesture that says the day is done to sit in the same \
             dock as the practice one, got: {html}"
        );
    }

    #[test]
    fn before_today_is_recorded_the_done_gesture_reads_as_the_offer() {
        let html = render(RootAtKnownHabit);

        assert!(
            html.contains("aria-label=\"C&#39;est fait · Lire une page\""),
            "expected the offer to record today, got: {html}"
        );
        assert!(
            !html.contains("Fait aujourd"),
            "expected no recorded-today wording before the gesture has been used, got: {html}"
        );
    }

    #[test]
    fn once_today_is_recorded_the_done_gesture_states_the_fact() {
        let html = render(RootAtFloorHabitDoneToday);

        assert!(
            html.contains("aria-label=\"Fait aujourd&#39;hui · Lire une page\""),
            "expected the recorded fact to be announced, got: {html}"
        );
        assert!(
            html.contains(r#"class="btn btn-secondary action-done is-done""#)
                && html.contains(r#"class="action-done-check""#)
                && html.contains(">C&#39;est fait<"),
            "expected the done state to carry its own shape cue and to keep the \
             owner's words, got: {html}"
        );
        assert!(
            html.contains(
                r#"<svg class="action-done-check" viewBox="0 0 24 24" aria-hidden="true" focusable="false">"#
            ),
            "expected the check to stay decorative — the gesture's own words \
             already carry the fact, got: {html}"
        );
    }

    // @scenario: mark-done/S4
    #[test]
    fn saying_its_done_from_the_habit_s_own_screen_records_today_without_the_ritual() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("C'est fait · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("aria-label=\"Fait aujourd&#39;hui · Lire une page\""),
            "expected the day to read as recorded from the habit's own screen, got: {html}"
        );
        assert!(
            html.contains(r#"class="screen detail""#) && !html.contains(r#"class="screen ritual""#),
            "expected the gesture to happen on the habit's screen, never by \
             entering the ritual, got: {html}"
        );
    }

    #[test]
    fn the_paused_and_anchored_docks_carry_no_done_gesture() {
        let paused = render(RootAtPausedHabit);
        let anchored = render(RootAtAnchoredHabit);

        assert!(
            !paused.contains("C&#39;est fait"),
            "expected no done gesture on a paused habit, got: {paused}"
        );
        assert!(
            !anchored.contains("C&#39;est fait"),
            "expected no done gesture on an anchored habit, got: {anchored}"
        );
    }

    #[test]
    fn the_paused_actions_resume_gesture_lives_in_a_dock_too() {
        let html = render(RootAtPausedHabit);

        assert!(
            html.contains(r#"class="action-dock""#),
            "expected the resume gesture to live in the screen's dock, got: {html}"
        );
        assert!(
            html.contains(r#"class="btn btn-primary btn-block""#)
                && html.contains(">La reprendre<"),
            "expected the docked resume gesture to keep its pill and its label, got: {html}"
        );
    }

    #[test]
    fn the_pace_zone_is_a_heading_over_two_tiles_with_their_glyphs() {
        let html = render(RootAtKnownHabit);

        assert!(
            html.contains(r#"<h2 class="pace-heading">Ajuster, à votre rythme</h2>"#),
            "expected the pace heading as an h2, got: {html}"
        );
        assert_eq!(
            html.matches(r#"class="pace-tile is-grow""#).count(),
            1,
            "expected one growing tile, got: {html}"
        );
        assert_eq!(
            html.matches(r#"class="pace-tile is-lighten""#).count(),
            1,
            "expected one lightening tile, got: {html}"
        );
        assert!(
            html.contains(r#"class="pace-glyph""#) && html.contains(">+<") && html.contains(">−<"),
            "expected each tile to carry its decorative glyph, got: {html}"
        );
        assert!(
            html.contains(">Passer à 6 min<") && html.contains(">Alléger à 4 min<"),
            "expected the tile labels unchanged, got: {html}"
        );
    }

    #[test]
    fn the_end_of_life_gestures_are_lines_under_a_fine_rule() {
        let html = render(RootAtKnownHabit);

        assert!(
            html.contains(r#"class="lifecycle""#),
            "expected the end-of-life lines to sit in one block, got: {html}"
        );
        assert_eq!(
            html.matches(r#"class="lifecycle-gesture""#).count(),
            1,
            "expected one plain end-of-life line, got: {html}"
        );
        assert_eq!(
            html.matches(r#"class="lifecycle-gesture is-anchor""#)
                .count(),
            1,
            "expected the anchoring line to carry the ocre modifier, got: {html}"
        );
        assert!(
            html.contains(">Mettre en pause, sans culpabilité<")
                && html.contains(">L&#39;ancrer · elle est devenue naturelle<"),
            "expected the end-of-life gestures' copy unchanged, got: {html}"
        );
        assert!(
            html.contains("aria-label=\"Mettre en pause, sans culpabilité · Lire une page\"")
                && html.contains(
                    "aria-label=\"L&#39;ancrer · elle est devenue naturelle · Lire une page\""
                ),
            "expected the click handles unchanged, got: {html}"
        );
    }

    // Retagged from a Task 1 mistag: this test's Given ("a paused habit")
    // and Then (offers only the return, plus the staircase, neither ritual,
    // growing, nor lightening) are S4's verbatim, not S1's (S1 is about the
    // Today screen leaving the daily list, anchored separately in
    // list_board_habits.rs and today.rs).
    // @scenario: pause-resume/S4
    #[test]
    fn a_paused_habits_detail_offers_only_its_return_and_staircase() {
        let html = render(RootAtPausedHabit);

        assert!(
            html.contains("La reprendre"),
            "expected the resume gesture to be offered, got: {html}"
        );
        assert_eq!(
            html.matches("day-pebble").count(),
            7,
            "expected the practice week to stay on a paused habit, got: {html}"
        );
        assert!(
            !html.contains("Passer à"),
            "expected no grow-goal gesture on a paused habit, got: {html}"
        );
        assert!(
            !html.contains("Alléger à"),
            "expected no lighten-goal gesture on a paused habit, got: {html}"
        );
        assert!(
            !html.contains("Commencer ma pratique"),
            "expected no ritual gesture on a paused habit, got: {html}"
        );
        assert!(
            !html.contains("Mettre en pause"),
            "expected no pause gesture on an already-paused habit, got: {html}"
        );
    }

    #[test]
    fn an_anchored_habits_detail_shows_the_banner_and_staircase_with_no_gesture() {
        let html = render(RootAtAnchoredHabit);

        assert!(
            html.contains("ancrée · 5 min"),
            "expected the anchored banner naming the dose, got: {html}"
        );
        assert_eq!(
            html.matches("day-pebble").count(),
            7,
            "expected the practice week to stay on an anchored habit, got: {html}"
        );
        assert!(
            !html.contains("Passer à") && !html.contains("Alléger à"),
            "expected no goal-adjustment gesture on an anchored habit, got: {html}"
        );
        assert!(
            !html.contains("Commencer ma pratique"),
            "expected no ritual gesture on an anchored habit, got: {html}"
        );
        assert!(
            !html.contains("Mettre en pause") && !html.contains("La reprendre"),
            "expected no pause/resume gesture on an anchored habit, got: {html}"
        );
        assert!(
            !html.contains("L&#39;ancrer"),
            "expected no anchor gesture on an already-anchored habit, got: {html}"
        );
    }

    // @scenario: anchor-habit/S4
    #[test]
    fn anchoring_is_offered_but_never_suggested_whatever_the_habits_history() {
        let html = render(RootAtHabitDoneTenOfLastFourteenDays);

        assert!(
            html.contains("L&#39;ancrer"),
            "expected the anchor gesture to still be offered, got: {html}"
        );
        let lowercase_html = html.to_lowercase();
        assert!(
            !lowercase_html.contains("suggé")
                && !lowercase_html.contains("prête")
                && !lowercase_html.contains("badge")
                && !lowercase_html.contains("stable"),
            "expected no suggestion, hint or badge about anchoring — anchoring is \
             user-initiated only, never detected, got: {html}"
        );
    }

    #[test]
    fn an_unknown_habit_shows_a_quiet_fallback_with_a_link_back() {
        let html = render(RootAtUnknownHabit);

        assert!(
            !html.contains("day-pebble"),
            "expected no week of pebbles for a missing habit, got: {html}"
        );
        assert!(
            html.contains("Cette habitude") && html.contains("plus sur votre liste"),
            "expected the quiet not-found copy, got: {html}"
        );
        assert!(
            html.contains("Aujourd") && html.contains("quiet-link"),
            "expected a link back to Aujourd'hui, got: {html}"
        );
    }

    // Test List — week pebble-size normalization (fix/practice-staircase-
    // overflow, owner ruling 2026-08-21). Each pebble's size is relative to
    // its own window's tallest goal, never an absolute minute value (the
    // same normalization the Week screen uses — owner ruling, 2026-08-21).
    // No scenario in practice-staircase.feature names normalization
    // directly — it is a rendering concern the feature's Given/When/Then
    // are silent on — so
    // these tests are left unanchored, each with a comment stating why.
    // - a flat window (goal never changed) normalizes every pebble to 1.0.
    // - a window whose goal grew mid-way normalizes on the window's own
    //   maximum, ascending ratios, last pebble at 1.0.
    // - a window whose maximum sits mid-history (grown then lightened, not
    //   the last day) still normalizes on that maximum, never on the current
    //   goal — no pebble may exceed the tallest one.
    // - a purely descending window (lightened once, maximum on the first
    //   day only) still normalizes on that first-day maximum.
    // - a window whose maximum sits on the last day only (grown today, the
    //   single most common gesture right before opening the detail)
    //   still normalizes on that last-day maximum.

    // Unanchored: no scenario names normalization; S5 (window is seven days)
    // is already pinned by the day-pebble count tests above. RootAtKnownHabit's
    // habit never grows, so every one of its seven days shares the same goal
    // (5) — the window's maximum equals every day's goal.
    #[test]
    fn a_flat_window_normalizes_every_pebble_to_full_size() {
        let html = render(RootAtKnownHabit);

        assert_eq!(
            day_pebble_ratios(&html),
            vec![1.0; 7],
            "expected every pebble to reach full size when the goal never \
             changed across the window, got: {html}"
        );
    }

    fn services_with_a_habit_grown_mid_window() -> Services {
        let today = LocalDate::from_epoch_day(20_020);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        habit.grow(LocalDate::from_epoch_day(20_018));
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtHabitGrownMidWindow() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_grown_mid_window);
        rsx! {
            Router::<Route> {}
        }
    }

    // Unanchored: see the Test List comment above. Window is 20_014..=20_020;
    // grow(20_018) raises the goal from the 20_018 onward, so the earlier
    // four days keep the starting goal (5) and the later three reach the new
    // one (6) — the window's own maximum.
    #[test]
    fn a_window_grown_mid_way_normalizes_ascending_to_its_own_maximum() {
        let html = render(RootAtHabitGrownMidWindow);

        assert_eq!(
            day_pebble_ratios(&html),
            vec![5.0 / 6.0, 5.0 / 6.0, 5.0 / 6.0, 5.0 / 6.0, 1.0, 1.0, 1.0],
            "expected the days before the growth to sit below full size and \
             the days at or after it to reach it, got: {html}"
        );
    }

    fn services_with_a_habit_grown_then_lightened_mid_window() -> Services {
        let today = LocalDate::from_epoch_day(20_020);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        habit.grow(LocalDate::from_epoch_day(20_016));
        habit.lighten(LocalDate::from_epoch_day(20_018));
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtHabitGrownThenLightenedMidWindow() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_grown_then_lightened_mid_window);
        rsx! {
            Router::<Route> {}
        }
    }

    // Unanchored: see the Test List comment above. `LightenGoal` is a
    // delivered, wired use case (issue #13), so a window whose maximum sits
    // mid-history — grown then lightened back down, not the current goal —
    // is reachable today. Window is 20_014..=20_020; grow(20_016) raises the
    // goal to 6 from 20_016, lighten(20_018) lowers it back to 5 from
    // 20_018, so the window's own maximum (6) is reached only on 20_016 and
    // 20_017, neither the first nor the last day. Pins both the
    // anti-overflow invariant and the exact normalization target — the
    // invariant is what a `.max()` -> `.last()` mutant breaks (see
    // mutation-report).
    #[test]
    fn a_window_grown_then_lightened_still_normalizes_on_its_own_maximum() {
        let html = render(RootAtHabitGrownThenLightenedMidWindow);

        let ratios = day_pebble_ratios(&html);
        assert!(
            ratios.iter().all(|&ratio| ratio <= 1.0),
            "no pebble may exceed the tallest one: {ratios:?}"
        );
        assert_eq!(
            ratios,
            vec![
                5.0 / 6.0,
                5.0 / 6.0,
                1.0,
                1.0,
                5.0 / 6.0,
                5.0 / 6.0,
                5.0 / 6.0
            ],
            "expected the window's own maximum (6, reached on days 3-4) to \
             normalize every pebble, not the current goal (5, which the window \
             lightened back down to), got: {html}"
        );
    }

    fn services_with_a_habit_lightened_early_in_window() -> Services {
        let today = LocalDate::from_epoch_day(20_020);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(6).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        habit.lighten(LocalDate::from_epoch_day(20_015));
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtHabitLightenedEarlyInWindow() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_lightened_early_in_window);
        rsx! {
            Router::<Route> {}
        }
    }

    // Unanchored: see the Test List comment above. Window is 20_014..=20_020;
    // the habit starts at goal 6 and lighten(20_015) lowers it to 5 from
    // that day onward, so only the first day (20_014) keeps the starting
    // goal — the window's own maximum sits on the first day alone, a
    // purely descending profile. A `.max()` taken over
    // `days.iter().skip(1)` would miss it entirely and overflow the first
    // pebble's size.
    #[test]
    fn a_window_only_lightened_normalizes_on_its_first_day() {
        let html = render(RootAtHabitLightenedEarlyInWindow);

        let ratios = day_pebble_ratios(&html);
        assert!(
            ratios.iter().all(|&ratio| ratio <= 1.0),
            "no pebble may exceed the tallest one: {ratios:?}"
        );
        assert_eq!(
            ratios,
            vec![
                1.0,
                5.0 / 6.0,
                5.0 / 6.0,
                5.0 / 6.0,
                5.0 / 6.0,
                5.0 / 6.0,
                5.0 / 6.0
            ],
            "expected the first day's goal (6) to normalize every pebble, got: {html}"
        );
    }

    fn services_with_a_habit_grown_on_the_last_day() -> Services {
        let today = LocalDate::from_epoch_day(20_020);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        habit.grow(LocalDate::from_epoch_day(20_020));
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtHabitGrownOnTheLastDay() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_grown_on_the_last_day);
        rsx! {
            Router::<Route> {}
        }
    }

    // Unanchored: see the Test List comment above. Window is 20_014..=20_020;
    // the habit starts at goal 5 and grow(20_020) raises it to 6 only from
    // today — the window's own maximum sits on the last day alone, the most
    // common gesture right before opening the detail.
    // `a_window_grown_mid_way_normalizes_ascending_to_its_own_maximum`
    // places the maximum on the last THREE days and does not discriminate
    // this narrower case.
    #[test]
    fn a_window_grown_on_its_last_day_normalizes_on_that_last_day() {
        let html = render(RootAtHabitGrownOnTheLastDay);

        assert_eq!(
            day_pebble_ratios(&html),
            vec![
                5.0 / 6.0,
                5.0 / 6.0,
                5.0 / 6.0,
                5.0 / 6.0,
                5.0 / 6.0,
                5.0 / 6.0,
                1.0
            ],
            "expected the last day's raised goal (6) to normalize every pebble, got: {html}"
        );
    }
}
