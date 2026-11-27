use crate::composition::Services;
use crate::i18n::tr;
use crate::route::Route;
use crate::views::back_link::BackLink;
use dioxus::prelude::*;
use kayzen_core::habit_management::queries::get_habit_detail::HabitDetail as HabitDetailData;
use kayzen_core::habit_management::queries::get_habit_detail::HabitState;
use kayzen_core::habit_management::queries::get_habit_detail::PracticeDay;
use kayzen_core::shared::clock::weekday_in_window;

#[component]
pub fn HabitDetail(id: String) -> Element {
    let services = use_context::<Services>();
    let today_weekday = services.today_calendar_parts().weekday;
    let mut detail = use_signal({
        let services = services.clone();
        let id = id.clone();
        move || services.get_habit_detail.handle(&id)
    });

    match detail() {
        Some(habit) => {
            let back_link = rsx! { BackLink {} };

            let week = rsx! {
                div {
                    class: "week-card",
                    role: "img",
                    "aria-label": tr!("staircase-aria", goal: habit.current_goal as i64),
                    div { class: "pebble-track",
                        for (day_offset, (day, ratio)) in
                            habit.days.iter().zip(day_ratios(&habit.days)).enumerate()
                        {
                            span { key: "{day_offset}", class: "pebble-day",
                                span {
                                    class: day_pebble_class(day.done, day_offset + 1 == habit.days.len()),
                                    style: "--day-ratio: {ratio}",
                                }
                                span { class: "pebble-day-name", "aria-hidden": "true",
                                    {tr!(
                                        "weekday-short",
                                        weekday: weekday_in_window(today_weekday, habit.days.len(), day_offset) as i64
                                    )}
                                }
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
                            div { class: "dock-pills",
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
                            if habit.attempts_today >= 1 {
                                div { class: "dock-tally",
                                    span { class: "quiet-note", {tr!("habit-detail-today-count", count: habit.attempts_today as i64)} }
                                    button {
                                        class: "resume-link action-undo",
                                        aria_label: tr!("habit-detail-undo-aria", title: habit.title.clone()),
                                        onclick: {
                                            let services = services.clone();
                                            let id = id.clone();
                                            move |_| detail.set(undo_attempt_and_reload(&services, &id))
                                        },
                                        {tr!("habit-detail-undo-label")}
                                    }
                                }
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

                        p { class: "quiet-note", {tr!("habit-detail-anchored-return")} }
                    }
                },
            }
        }
        None => rsx! {
            div { class: "screen",
                h1 { class: "greeting", {tr!("habit-not-found-message")} }
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
fn undo_attempt_and_reload(services: &Services, id: &str) -> Option<HabitDetailData> {
    services.undo_attempt.execute(id).ok();
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

// @algo: a day's attempts against its own goal, capped at 1.0 (CSS has no clamp); no attempt is 0.0, the floor.
#[must_use]
fn day_ratios(days: &[PracticeDay]) -> Vec<f64> {
    days.iter()
        .map(|day| (day.attempts as f64 / day.goal as f64).min(1.0))
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
        habit.record_attempt(clock.today());
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
            habit.record_attempt(today.minus_days(days_back));
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
            html.contains(r#"class="dose">5 times a day<"#),
            "expected the active-dose line in English, got: {html}"
        );
        assert!(
            !html.contains(" min") && !html.contains("minute"),
            "expected the goal in times, never minutes, got: {html}"
        );
        assert!(
            html.contains(r#"class="pace-heading">Adjust, at your own pace<"#),
            "expected the pace heading in English, got: {html}"
        );
        assert!(
            html.contains(">Start<"),
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
    fn RootAtFloorHabitDoneTodayAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_floor_habit_done_today);
        rsx! {
            Router::<Route> {}
        }
    }

    #[test]
    fn an_english_floor_habit_reads_its_singular_time_on_both_lines() {
        let html = render(RootAtFloorHabitDoneTodayAndEnglishLocale);

        assert!(
            html.contains(r#"class="dose">1 time a day<"#),
            "expected the singular dose line in English, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Your last seven days, current goal 1 time a day""#),
            "expected the singular staircase label in English, got: {html}"
        );
        assert!(
            html.contains(">Lighten to 1 time<"),
            "expected the floor to read 'Lighten to 1 time', got: {html}"
        );
        assert!(
            !html.contains(" min") && !html.contains("minute"),
            "expected the goal in times, never minutes, got: {html}"
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

        screen.click("Passer à 6 fois · Lire une page");

        let html = screen.html();
        assert!(
            html.contains(r#"class="dose">6 fois par jour<"#),
            "expected the raised goal to appear after the click, got: {html}"
        );
    }

    #[test]
    fn clicking_lighten_lowers_the_goal_and_re_renders_the_dose() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("Alléger à 4 fois · Lire une page");

        let html = screen.html();
        assert!(
            html.contains(r#"class="dose">4 fois par jour<"#),
            "expected the lowered goal to appear after the click, got: {html}"
        );
    }

    #[test]
    fn clicking_pause_re_renders_the_paused_banner() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("Mettre en pause, sans culpabilité · Lire une page");

        let html = screen.html();
        assert!(
            html.contains(r#"class="dose">en pause · 5 fois par jour<"#),
            "expected the paused banner after the click, got: {html}"
        );
    }

    #[test]
    fn clicking_anchor_re_renders_the_anchored_banner() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("L'ancrer · elle est devenue naturelle · Lire une page");

        let html = screen.html();
        assert!(
            html.contains(r#"class="dose">ancrée · 5 fois par jour<"#),
            "expected the anchored banner after the click, got: {html}"
        );
    }

    #[test]
    fn clicking_la_reprendre_re_renders_the_active_dose() {
        let mut screen = Screen::open(RootAtPausedHabit);

        screen.click("La reprendre · Lire une page");

        let html = screen.html();
        assert!(
            html.contains(r#"class="dose">5 fois par jour<"#),
            "expected the active dose to re-render after the click, got: {html}"
        );
        assert!(
            !html.contains(r#"class="dose">en pause"#),
            "expected the paused dose to be gone after resuming, got: {html}"
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
    fn mark_done_and_reload_adds_one_attempt_and_returns_the_refreshed_detail() {
        let today = LocalDate::from_epoch_day(20_005);
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        let services =
            Services::with_repository_and_clock(Rc::clone(&repository), Rc::new(FixedClock(today)));

        let detail = mark_done_and_reload(&services, "h-1");

        let attempts = repository
            .get(&HabitId::new("h-1").unwrap())
            .unwrap()
            .attempts_on(today);
        assert_eq!(
            (attempts, detail.map(|d| d.done_today)),
            (1, Some(false)),
            "expected one attempt recorded, not yet the goal of five, before the screen re-reads"
        );
    }

    #[test]
    fn taps_through_the_detail_gesture_count_each_time_and_the_day_is_done_at_the_goal() {
        let services = services_with_one_habit();

        let after_each_tap: Vec<_> = (0..6)
            .map(|_| mark_done_and_reload(&services, "h-1").map(|d| d.done_today))
            .collect();

        assert_eq!(
            after_each_tap,
            [false, false, false, false, true, true].map(Some),
            "expected every tap to add an attempt, done from the fifth, never cleared"
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
            html.contains(r#"class="dose">5 fois par jour<"#),
            "expected the dose, got: {html}"
        );
        assert!(
            !html.contains(" min") && !html.contains("minute"),
            "expected the goal in times, never minutes, got: {html}"
        );
        assert!(
            html.contains("Passer à 6 fois"),
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
            html.contains("Commencer"),
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
            html.contains("Passer à 2 fois"),
            "expected the grow-goal button even at the floor, got: {html}"
        );
        assert!(
            html.contains("Alléger à 1 fois"),
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
            html.contains(
                r#"role="img" aria-label="Vos sept derniers jours, objectif actuel 5 fois par jour""#
            ),
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

    fn pebble_day_names(html: &str) -> Vec<&str> {
        const NEEDLE: &str = r#"<span class="pebble-day-name" aria-hidden="true">"#;
        html.split(NEEDLE)
            .skip(1)
            .map(|rest| rest.split('<').next().expect("a name ends at its tag"))
            .collect()
    }

    fn services_with_one_habit_on_a_monday() -> Services {
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_000)));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtHabitOnAMondayInEnglish() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_habit_on_a_monday);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtHabitOnAMonday() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_habit_on_a_monday);
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: practice-staircase/S8
    #[test]
    fn each_pebble_names_its_weekday_in_french() {
        let html = render(RootAtHabitOnAMonday);

        assert_eq!(
            pebble_day_names(&html),
            ["ma", "me", "je", "ve", "sa", "di", "lu"],
            "expected the window ending on a Monday to name its days under the pebbles, got: {html}"
        );
        assert!(
            html.contains(
                r#"role="img" aria-label="Vos sept derniers jours, objectif actuel 5 fois par jour""#
            ),
            "the week card's label stays the one that already describes the week, got: {html}"
        );
    }

    // @scenario: practice-staircase/S8
    #[test]
    fn each_pebble_names_its_weekday_in_english() {
        let html = render(RootAtHabitOnAMondayInEnglish);

        assert_eq!(
            pebble_day_names(&html),
            ["Tu", "We", "Th", "Fr", "Sa", "Su", "Mo"],
            "expected the same window in English, got: {html}"
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
                && html.contains(">Commencer<"),
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
                && dock_actions.contains(">Commencer<"),
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
            html.contains(
                "aria-label=\"C&#39;est fait · objectif du jour atteint · Lire une page\""
            ),
            "expected the spoken name to contain the visible label and the recorded fact, got: {html}"
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
    fn saying_its_done_from_the_habit_s_own_screen_adds_an_attempt_without_the_ritual() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("C'est fait · Lire une page");

        let html = screen.html();
        assert!(
            !html.contains("Fait aujourd"),
            "expected one attempt of five not to read as done, got: {html}"
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
            html.contains(">Passer à 6 fois<") && html.contains(">Alléger à 4 fois<"),
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
            !html.contains("Commencer"),
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
            html.contains(r#"class="dose">ancrée · 5 fois par jour<"#),
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
            !html.contains("Commencer"),
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

    #[component]
    fn RootAtAnchoredHabitAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_anchored_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: anchor-habit/S6
    #[test]
    fn an_anchored_habits_detail_says_how_it_comes_back_in_french() {
        let html = render(RootAtAnchoredHabit);

        assert!(
            html.contains(r#"<p class="quiet-note">Elle peut revenir dans votre quotidien depuis Ancrées.</p>"#),
            "expected one quiet line saying how the habit comes back, got: {html}"
        );
        assert_eq!(
            html.matches("<a ").count(),
            1,
            "expected only the back link, no new link, got: {html}"
        );
        assert_eq!(
            html.matches("<button").count(),
            0,
            "expected no button on an anchored habit's detail, got: {html}"
        );
    }

    // @scenario: anchor-habit/S6
    #[test]
    fn an_anchored_habits_detail_says_how_it_comes_back_in_english() {
        let html = render(RootAtAnchoredHabitAndEnglishLocale);

        assert!(
            html.contains(
                r#"<p class="quiet-note">It can come back to your daily life from Anchored.</p>"#
            ),
            "expected the same quiet line in English, naming the Anchored nav label, got: {html}"
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
            html.contains("Retour à aujourd") && html.contains("quiet-link"),
            "expected a link back to Aujourd'hui, got: {html}"
        );
    }

    // @scenario: practice-staircase/S2
    #[test]
    fn every_undone_pebble_stays_at_floor_size() {
        let html = render(RootAtKnownHabit);

        assert_eq!(
            day_pebble_ratios(&html),
            vec![0.0; 7],
            "expected every pebble to stay at the floor when no day was \
             practised, got: {html}"
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
        for _ in 0..2 {
            habit.record_attempt(LocalDate::from_epoch_day(20_015));
        }
        for _ in 0..3 {
            habit.record_attempt(LocalDate::from_epoch_day(20_019));
        }
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

    #[test]
    fn a_window_grown_mid_way_sizes_each_practised_day_against_its_own_goal() {
        let html = render(RootAtHabitGrownMidWindow);

        assert_eq!(
            day_pebble_ratios(&html),
            vec![0.0, 2.0 / 5.0, 0.0, 0.0, 0.0, 3.0 / 6.0, 0.0],
            "expected 2 attempts on goal 5 and 3 attempts on the grown goal 6, \
             the other days at the floor, got: {html}"
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
        for _ in 0..3 {
            habit.record_attempt(LocalDate::from_epoch_day(20_017));
            habit.record_attempt(LocalDate::from_epoch_day(20_019));
        }
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

    #[test]
    fn a_window_grown_then_lightened_sizes_each_practised_day_against_its_own_goal() {
        let html = render(RootAtHabitGrownThenLightenedMidWindow);

        assert_eq!(
            day_pebble_ratios(&html),
            vec![0.0, 0.0, 0.0, 3.0 / 6.0, 0.0, 3.0 / 5.0, 0.0],
            "expected 3 attempts on the goal 6 of the grown days and 3 \
             attempts on the lightened goal 5, got: {html}"
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
        for _ in 0..3 {
            habit.record_attempt(LocalDate::from_epoch_day(20_014));
        }
        for _ in 0..4 {
            habit.record_attempt(LocalDate::from_epoch_day(20_016));
        }
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

    #[test]
    fn a_window_only_lightened_sizes_each_practised_day_against_its_own_goal() {
        let html = render(RootAtHabitLightenedEarlyInWindow);

        assert_eq!(
            day_pebble_ratios(&html),
            vec![3.0 / 6.0, 0.0, 4.0 / 5.0, 0.0, 0.0, 0.0, 0.0],
            "expected 3 attempts on the first day's goal 6 and 4 attempts on \
             the lightened goal 5, got: {html}"
        );
    }

    fn services_with_attempts_on_three_days() -> Services {
        let today = LocalDate::from_epoch_day(20_020);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        for (days_back, attempts) in [(0, 2), (1, 7), (2, 5)] {
            for _ in 0..attempts {
                habit.record_attempt(today.minus_days(days_back));
            }
        }
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtAttemptsOnThreeDays() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_attempts_on_three_days);
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: practice-staircase/S4
    #[test]
    fn a_practised_day_is_sized_by_its_attempts_against_its_goal_and_capped_at_full() {
        let html = render(RootAtAttemptsOnThreeDays);

        assert_eq!(
            day_pebble_ratios(&html),
            vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.4],
            "goal 5: 5 attempts reach full size, 7 attempts stay at full \
             size, 2 attempts stand at two fifths, and undone days stay at \
             the floor, got: {html}"
        );
    }

    // @scenario: practice-staircase/S4
    #[test]
    fn a_practised_pebble_is_drawn_at_its_ratio_and_an_undone_one_at_the_floor() {
        const MAIN_CSS: &str = include_str!("../../assets/main.css");

        assert!(
            MAIN_CSS.contains(
                ".day-pebble.is-done {\n    --pebble-scale: max(.25, var(--day-ratio, 1));"
            ),
            "a practised pebble must scale with its ratio (2 of 5 draws at two fifths), above a visible minimum"
        );
        assert!(
            MAIN_CSS.contains(".day-pebble {\n    --pebble-scale: .6;"),
            "an undone pebble keeps its hollow ring at the unchanged floor"
        );
    }

    fn services_with_one_attempt_today() -> Services {
        let today = LocalDate::from_epoch_day(20_020);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        habit.record_attempt(today);
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtOneAttemptToday() -> Element {
        crate::i18n::use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_attempt_today);
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: undo-attempt/S4
    #[test]
    fn the_undo_control_is_shown_only_when_today_has_an_attempt() {
        let none = render(RootAtKnownHabit);
        let one = render(RootAtOneAttemptToday);

        assert!(
            !none.contains("Retirer une fois"),
            "expected no undo control at 0 attempts today, got: {none}"
        );
        assert!(
            one.contains("Retirer une fois"),
            "expected the undo control at 1 attempt today, got: {one}"
        );
        assert!(
            !one.contains("quiet-link"),
            "expected the undo control not to be a quiet-link, got: {one}"
        );
    }

    #[test]
    fn with_an_attempt_the_dock_keeps_its_two_pills_on_row_one_and_the_count_and_undo_on_a_quiet_row_two()
     {
        let none = render(RootAtKnownHabit);
        let one = render(RootAtOneAttemptToday);

        assert!(
            !none.contains("dock-tally"),
            "expected no second row at 0 attempts today, got: {none}"
        );
        let pills = one
            .find(r#"class="dock-pills""#)
            .expect("expected the two pills on their own row");
        let tally = one
            .find(r#"class="dock-tally""#)
            .expect("expected the count and the undo on a second row");
        let done = one
            .find(">C&#39;est fait<")
            .expect("expected the done pill");
        let count = one
            .find(">1 fois aujourd&#39;hui<")
            .expect("expected the count");
        assert!(
            pills < done && done < tally && tally < count,
            "expected the pills first, then the tally row holding the count, got: {one}"
        );
        assert!(
            one.contains(r#"class="resume-link action-undo""#)
                && !one.contains("btn btn-secondary action-undo"),
            "expected the undo to be a quiet text gesture, not a pill, got: {one}"
        );
    }

    #[test]
    fn undoing_from_the_habit_s_own_screen_lowers_today_s_count_by_one() {
        let mut screen = Screen::open(RootAtAttemptsOnThreeDays);

        screen.click("Retirer une fois · Lire une page");

        let html = screen.html();
        assert_eq!(
            day_pebble_ratios(&html),
            vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.2],
            "expected today's 2 attempts of goal 5 to read as 1 and the other \
             days to stay as they were, got: {html}"
        );
        assert!(
            html.contains("Retirer une fois"),
            "expected the control to stay while an attempt remains, got: {html}"
        );
    }

    #[test]
    fn undoing_the_only_attempt_hides_the_control_again() {
        let mut screen = Screen::open(RootAtOneAttemptToday);

        screen.click("Retirer une fois · Lire une page");

        let html = screen.html();
        assert!(
            !html.contains("Retirer une fois"),
            "expected the control to leave once today has no attempt, got: {html}"
        );
    }

    #[component]
    fn RootAtOneAttemptTodayAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_attempt_today);
        rsx! {
            Router::<Route> {}
        }
    }

    #[test]
    fn a_tap_on_the_done_gesture_shows_todays_count_beside_it() {
        let mut screen = Screen::open(RootAtKnownHabit);

        screen.click("C'est fait · Lire une page");

        let html = screen.html();
        assert!(
            html.contains(">1 fois aujourd&#39;hui<"),
            "expected the count of today's attempts beside the done gesture after one tap, got: {html}"
        );
    }

    #[test]
    fn a_tap_that_adds_a_second_attempt_updates_the_count_in_english() {
        let mut screen = Screen::open(RootAtOneAttemptTodayAndEnglishLocale);

        screen.click("It's done · Lire une page");

        let html = screen.html();
        assert!(
            html.contains(">2 times today<"),
            "expected the count to read 2 times today after the second attempt, got: {html}"
        );
    }

    #[test]
    fn the_count_line_is_absent_when_today_has_no_attempt() {
        let html = render(RootAtKnownHabit);

        assert!(
            !html.contains("fois aujourd&#39;hui<"),
            "expected no count line at 0 attempts today, got: {html}"
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

    // @scenario: practice-staircase/S3
    #[test]
    fn a_goal_grown_on_an_undone_last_day_leaves_that_day_at_the_floor() {
        let html = render(RootAtHabitGrownOnTheLastDay);

        assert_eq!(
            day_pebble_ratios(&html),
            vec![0.0; 7],
            "expected growing the goal to draw nothing until the day is \
             practised, got: {html}"
        );
    }
}
