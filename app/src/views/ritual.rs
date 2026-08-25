use crate::composition::Services;
use crate::i18n::tr;
use crate::route::Route;
use dioxus::prelude::*;
use dioxus::router::Navigator;
use kayzen_core::habit_management::queries::get_habit_detail::HabitState;
use std::time::Duration;
use web_time::Instant;

const RING_RADIUS: f64 = 54.0;

#[component]
pub fn Ritual(id: String) -> Element {
    let services = use_context::<Services>();
    let habit = use_hook({
        let services = services.clone();
        let id = id.clone();
        move || services.get_habit_detail.handle(&id)
    });

    match habit {
        None => rsx! {
            div { class: "screen",
                p { class: "lede", {tr!("ritual-not-found-message")} }
                Link { class: "quiet-link", to: Route::Today {}, {tr!("ritual-not-found-back-link")} }
            }
        },
        Some(habit) => match habit.state {
            HabitState::Paused => rsx! {
                div { class: "screen",
                    header { class: "masthead",
                        Link { class: "quiet-link", to: Route::Today {}, {tr!("masthead-back-to-today")} }
                    }
                    h1 { class: "greeting", "{habit.title}" }
                    p { class: "quiet-note", {tr!("ritual-paused-message")} }
                }
            },
            HabitState::Anchored => rsx! {
                div { class: "screen",
                    header { class: "masthead",
                        Link { class: "quiet-link", to: Route::Today {}, {tr!("masthead-back-to-today")} }
                    }
                    h1 { class: "greeting", "{habit.title}" }
                    p { class: "quiet-note", {tr!("ritual-anchored-message")} }
                }
            },
            HabitState::Active => rsx! {
                PracticeTimer {
                    id: habit.id.clone(),
                    title: habit.title.clone(),
                    goal_minutes: habit.current_goal,
                }
            },
        },
    }
}

#[component]
fn PracticeTimer(id: String, title: String, goal_minutes: u32) -> Element {
    let services = use_context::<Services>();
    let navigator = use_navigator();
    let started_at = use_hook(Instant::now);
    let mut now = use_signal(move || started_at);

    let total = total_seconds(goal_minutes);
    let elapsed = now().duration_since(started_at);
    let remaining = remaining_seconds(total, elapsed);

    rsx! {
        div { class: "screen ritual",
            h1 { class: "greeting", "{title}" }
            PracticeCountdown {
                title: title.clone(),
                total,
                remaining,
                on_tick: move |()| now.set(Instant::now()),
            }
            button {
                class: "btn btn-primary btn-block",
                aria_label: tr!("ritual-complete-aria", title: title.clone()),
                onclick: {
                    let services = services.clone();
                    let id = id.clone();
                    move |_| complete_and_go_home(&services, navigator, &id)
                },
                {tr!("ritual-complete-label")}
            }
            Link {
                class: "quiet-link",
                to: Route::HabitDetail { id: id.clone() },
                aria_label: tr!("ritual-stop-aria", title: title.clone()),
                {tr!("ritual-stop-label")}
            }
        }
    }
}

// Named for what it does, not for one of its three siblings: the dial, the
// at-zero note and the tick sensor are all just the countdown, and none
// alone. Everything here depends only on `remaining`/`total` and nothing
// needs a mounted `Router` (unlike `PracticeTimer`'s `Link`) — that split is
// what lets tests pin the at-zero branch and the live ring/countdown
// directly, with crafted `remaining` values, instead of only through a real
// wall-clock wait.
#[component]
fn PracticeCountdown(
    title: String,
    total: u64,
    remaining: u64,
    on_tick: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "ritual-dial",
            role: "timer",
            aria_label: tr!("ritual-timer-aria", title: title.clone()),
            div { class: "ritual-breath" }
            svg { class: "ritual-ring", view_box: "0 0 120 120",
                circle { class: "ritual-ring-track", r: "{RING_RADIUS}", cx: "60", cy: "60" }
                circle {
                    class: "ritual-ring-run",
                    r: "{RING_RADIUS}",
                    cx: "60",
                    cy: "60",
                    stroke_dasharray: "{ring_circumference()}",
                    stroke_dashoffset: "{ring_offset(remaining, total)}",
                }
            }
            div { class: "ritual-countdown", "{countdown_label(remaining)}" }
        }
        if remaining == 0 {
            p { class: "quiet-note", {tr!("ritual-gentle-word")} }
        }
        if remaining > 0 {
            div {
                class: "ritual-tick",
                aria_hidden: "true",
                onanimationiteration: move |_| on_tick.call(()),
            }
        }
    }
}

fn complete_and_go_home(services: &Services, navigator: Navigator, id: &str) {
    record_todays_practice(services, id);
    navigator.push(Route::Today {});
}

fn record_todays_practice(services: &Services, id: &str) {
    let not_yet_done = services
        .get_habit_detail
        .handle(id)
        .is_some_and(|habit| !habit.done_today);
    if not_yet_done {
        services.mark_done.execute(id).ok();
    }
}

#[must_use]
fn total_seconds(goal_minutes: u32) -> u64 {
    u64::from(goal_minutes) * 60
}

#[must_use]
fn remaining_seconds(total: u64, elapsed: Duration) -> u64 {
    total.saturating_sub(elapsed.as_secs())
}

#[must_use]
fn countdown_label(remaining: u64) -> String {
    format!("{}:{:02}", remaining / 60, remaining % 60)
}

#[must_use]
fn ring_circumference() -> f64 {
    2.0 * std::f64::consts::PI * RING_RADIUS
}

// `total` is never 0: it is `goal_minutes * 60` and `Goal::MIN == 1`, an
// invariant kayzen-core enforces, not re-checked here.
#[must_use]
fn ring_offset(remaining: u64, total: u64) -> f64 {
    ring_circumference() * (1.0 - remaining as f64 / total as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{use_locale_for_tests, use_locale_for_tests_as};
    use crate::views::click_harness::Screen;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use dioxus_i18n::unic_langid::langid;
    use kayzen_core::habit_management::domain::goal::Goal;
    use kayzen_core::habit_management::domain::habit::Habit;
    use kayzen_core::habit_management::domain::habit_id::HabitId;
    use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
    use kayzen_core::habit_management::domain::habit_title::HabitTitle;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use kayzen_core::shared::clock::Clock;
    use kayzen_core::shared::local_date::LocalDate;
    use std::rc::Rc;

    // `Screen::open` takes a bare `fn() -> Element` (no captures allowed), so a
    // test that must mutate the repository BETWEEN `open` and `click` (proving
    // the completion gesture reads fresh at click time, not a mount-time
    // snapshot) hands the repository to the root through this thread-local
    // rather than a closure capture. Scoped to this test module only;
    // production code never reads it.
    thread_local! {
        static SHARED_REPOSITORY: std::cell::RefCell<Option<Rc<dyn HabitRepository>>> =
            const { std::cell::RefCell::new(None) };
    }

    // Clears the thread-local on drop, including on an unwind (e.g. a panic
    // inside `screen.click`) — a plain trailing reset after `click` would
    // leave a stale `Some` behind if `click` itself panicked, silently
    // poisoning whichever test runs next on this thread.
    struct SharedRepositoryGuard;

    impl Drop for SharedRepositoryGuard {
        fn drop(&mut self) {
            SHARED_REPOSITORY.with(|cell| *cell.borrow_mut() = None);
        }
    }

    fn services_over_shared_repository() -> Services {
        let repository = SHARED_REPOSITORY.with(|cell| {
            cell.borrow()
                .clone()
                .expect("a repository was seeded before opening the screen")
        });
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_005)));
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtActiveHabitOverSharedRepository() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path(
                "/habit/h-1/ritual",
            )));
        });
        use_context_provider(services_over_shared_repository);
        rsx! {
            Router::<Route> {}
        }
    }

    struct FixedClock(LocalDate);

    impl Clock for FixedClock {
        fn today(&self) -> LocalDate {
            self.0
        }
    }

    fn a_habit_with_goal(goal: u32) -> Habit {
        Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(goal).unwrap(),
            LocalDate::from_epoch_day(20_000),
        )
    }

    fn services_with_an_active_habit(goal: u32) -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit_with_goal(goal));
        Services::with_repository(repository)
    }

    #[component]
    fn RootAtActiveHabitGoalFive() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path(
                "/habit/h-1/ritual",
            )));
        });
        use_context_provider(|| services_with_an_active_habit(5));
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtActiveHabitGoalFiveAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path(
                "/habit/h-1/ritual",
            )));
        });
        use_context_provider(|| services_with_an_active_habit(5));
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_ritual_screen_in_english() {
        let html = render(RootAtActiveHabitGoalFiveAndEnglishLocale);

        assert!(
            html.contains(r#"aria-label="Timer · Lire une page""#),
            "expected the timer aria-label in English, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Done · Lire une page""#) && html.contains(">Done<"),
            "expected the complete gesture in English, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Stop, that&#39;s okay · Lire une page""#)
                && html.contains(">Stop, that&#39;s okay<"),
            "expected the stop gesture in English, got: {html}"
        );
    }

    #[component]
    fn RootAtActiveHabitGoalThree() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path(
                "/habit/h-1/ritual",
            )));
        });
        use_context_provider(|| services_with_an_active_habit(3));
        rsx! {
            Router::<Route> {}
        }
    }

    fn services_with_a_paused_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit_with_goal(5);
        habit.pause().expect("a fresh habit is active");
        repository.save(&habit);
        Services::with_repository(repository)
    }

    fn services_with_an_anchored_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit_with_goal(5);
        habit.anchor().expect("a fresh habit is active");
        repository.save(&habit);
        Services::with_repository(repository)
    }

    #[component]
    fn RootAtPausedHabitRitual() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path(
                "/habit/h-1/ritual",
            )));
        });
        use_context_provider(services_with_a_paused_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtAnchoredHabitRitual() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path(
                "/habit/h-1/ritual",
            )));
        });
        use_context_provider(services_with_an_anchored_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtUnknownHabitRitual() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path(
                "/habit/missing/ritual",
            )));
        });
        use_context_provider(|| Services::with_repository(Rc::new(InMemoryHabitRepository::new())));
        rsx! {
            Router::<Route> {}
        }
    }

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    #[test]
    fn total_seconds_multiplies_minutes_by_sixty() {
        assert_eq!(total_seconds(5), 300);
        assert_eq!(total_seconds(3), 180);
    }

    #[test]
    fn remaining_seconds_subtracts_elapsed_from_total() {
        assert_eq!(remaining_seconds(300, Duration::ZERO), 300);
        assert_eq!(remaining_seconds(300, Duration::from_secs(90)), 210);
    }

    // @scenario: ritual/S6
    #[test]
    fn remaining_seconds_saturates_to_zero_without_underflowing_when_elapsed_exceeds_total() {
        assert_eq!(remaining_seconds(300, Duration::from_secs(300)), 0);
        assert_eq!(remaining_seconds(300, Duration::from_secs(400)), 0);
    }

    #[test]
    fn countdown_label_formats_as_minutes_colon_two_digit_seconds() {
        assert_eq!(countdown_label(300), "5:00");
        assert_eq!(countdown_label(180), "3:00");
        assert_eq!(countdown_label(210), "3:30");
        assert_eq!(countdown_label(65), "1:05");
        assert_eq!(countdown_label(0), "0:00");
    }

    #[test]
    fn ring_circumference_is_two_pi_times_the_ring_radius() {
        let expected = 2.0 * std::f64::consts::PI * RING_RADIUS;
        assert!((ring_circumference() - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn ring_offset_scales_linearly_from_full_dose_to_none_remaining() {
        assert_eq!(ring_offset(300, 300), 0.0);
        let midpoint = ring_offset(150, 300);
        assert!((midpoint - ring_circumference() / 2.0).abs() < f64::EPSILON);
        assert_eq!(ring_offset(0, 300), ring_circumference());
    }

    // @scenario: ritual/S1
    #[test]
    fn opening_the_ritual_counts_down_from_the_habits_own_goal_not_a_fixed_minute() {
        for (root, expected_label) in [
            (RootAtActiveHabitGoalFive as fn() -> Element, "5:00"),
            (RootAtActiveHabitGoalThree as fn() -> Element, "3:00"),
        ] {
            let html = render(root);
            assert!(
                html.contains(expected_label),
                "expected the countdown to start at the habit's own goal ({expected_label}), got: {html}"
            );
            assert!(
                html.contains("aria-label=\"Minuteur · Lire une page\""),
                "expected the dial to carry the habit's own name, got: {html}"
            );
        }
    }

    // @scenario: ritual/S3
    #[test]
    fn stopping_the_ritual_returns_to_the_detail_screen_recording_nothing() {
        let mut screen = Screen::open(RootAtActiveHabitGoalFive);

        screen.click("Arrêter, ce n'est pas grave · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("Commencer ma pratique"),
            "expected the habit detail screen (not Today) to render after stopping, got: {html}"
        );
        assert!(
            !html.contains("is-done"),
            "expected nothing recorded — stopping is not failing, got: {html}"
        );
    }

    // @scenario: ritual/S2
    #[test]
    fn completing_the_practice_from_the_ritual_records_the_day_and_returns_to_aujourdhui() {
        let mut screen = Screen::open(RootAtActiveHabitGoalFive);

        screen.click("J'ai terminé · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("Vos petits pas"),
            "expected the screen to return to Aujourd'hui after completing the practice, got: {html}"
        );
        assert!(
            html.contains("target is-done"),
            "expected today's practice to be recorded on the Aujourd'hui board, got: {html}"
        );
    }

    // @scenario: ritual/S7
    #[test]
    fn the_completion_gesture_reads_the_day_at_click_time_not_at_mount() {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit_with_goal(5));
        SHARED_REPOSITORY.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&repository)));
        let _guard = SharedRepositoryGuard;
        let mut screen = Screen::open(RootAtActiveHabitOverSharedRepository);

        // Mutated AFTER mount (the component already ran `use_hook`), BEFORE
        // the click — a mount-time snapshot cannot see this, only a click-time
        // read can.
        let mut habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        habit.toggle_done(LocalDate::from_epoch_day(20_005));
        repository.save(&habit);
        screen.click("J'ai terminé · Lire une page");

        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert!(
            habit.is_done_on(LocalDate::from_epoch_day(20_005)),
            "expected the gesture to see the day already recorded and leave it alone"
        );
    }

    // Test List — record_todays_practice (the completion gesture's guard,
    // surrounded from both directions per the mutation table):
    // - called twice from an empty day -> the day stays recorded (never
    //   un-marked by a second call).
    // - called once from an empty day -> the day becomes recorded.
    // @scenario: ritual/S7
    #[test]
    fn recording_todays_practice_twice_leaves_the_day_recorded() {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit_with_goal(5));
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_005)));
        let services = Services::with_repository_and_clock(Rc::clone(&repository), clock);

        record_todays_practice(&services, "h-1");
        record_todays_practice(&services, "h-1");

        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert!(
            habit.is_done_on(LocalDate::from_epoch_day(20_005)),
            "expected the day to stay recorded after the gesture ran twice"
        );
    }

    #[test]
    fn recording_todays_practice_records_it_when_the_day_is_empty() {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit_with_goal(5));
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_005)));
        let services = Services::with_repository_and_clock(Rc::clone(&repository), clock);

        record_todays_practice(&services, "h-1");

        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert!(
            habit.is_done_on(LocalDate::from_epoch_day(20_005)),
            "expected the day to be recorded after the gesture ran once on an empty day"
        );
    }

    // @scenario: ritual/S4
    #[test]
    fn a_paused_or_anchored_habit_offers_no_practice_when_its_ritual_is_reached_by_hand() {
        for (root, expected_message) in [
            (
                RootAtPausedHabitRitual as fn() -> Element,
                "Cette habitude se repose en ce moment. Elle vous attend, sans presser.",
            ),
            (
                RootAtAnchoredHabitRitual as fn() -> Element,
                "Cette habitude est devenue naturelle. Elle a quitté votre quotidien.",
            ),
        ] {
            let html = render(root);
            assert!(
                html.contains(expected_message),
                "expected the refusal copy, got: {html}"
            );
            assert!(
                !html.contains("ritual-dial"),
                "expected no dial offered to a habit at rest, got: {html}"
            );
            assert!(
                !html.contains("ritual-countdown"),
                "expected no countdown offered to a habit at rest, got: {html}"
            );
            assert!(
                !html.contains("J&#39;ai termin"),
                "expected no completion gesture offered to a habit at rest, got: {html}"
            );
            assert!(
                !html.contains("<button"),
                "expected no button at all offered to a habit at rest, got: {html}"
            );
        }
    }

    // @scenario: ritual/S5
    #[test]
    fn an_unknown_habit_at_the_ritual_address_lands_on_the_quiet_fallback_with_a_way_back() {
        let html = render(RootAtUnknownHabitRitual);

        assert!(
            html.contains("Cette habitude n&#39;est plus sur votre liste."),
            "expected the quiet fallback copy, got: {html}"
        );
        assert!(
            html.contains("Aujourd") && html.contains("quiet-link"),
            "expected a way back, got: {html}"
        );
    }

    // Test List — PracticeCountdown (deterministic, crafted remaining/total, no Router needed):
    // - a partial remaining renders the countdown FROM remaining, not total
    // - a partial remaining renders the ring offset computed from remaining/total,
    //   not a hardcoded constant
    // - remaining == 0 renders the "time's up" note; a positive remaining does not
    // - remaining > 0 renders the tick sensor; remaining == 0 does not (it must stop
    //   ticking once there is nothing left to count down)

    #[component]
    fn RootAtDialPartial() -> Element {
        use_locale_for_tests();
        rsx! {
            PracticeCountdown {
                title: "Lire une page".to_string(),
                total: 300,
                remaining: 210,
                on_tick: move |()| {},
            }
        }
    }

    #[component]
    fn RootAtDialZero() -> Element {
        use_locale_for_tests();
        rsx! {
            PracticeCountdown {
                title: "Lire une page".to_string(),
                total: 300,
                remaining: 0,
                on_tick: move |()| {},
            }
        }
    }

    #[test]
    fn the_dial_countdown_reflects_remaining_time_not_the_total_goal() {
        let html = render(RootAtDialPartial);
        assert!(
            html.contains("3:30"),
            "expected the countdown to read the remaining 3:30, got: {html}"
        );
    }

    // "Minuteur" is a stable, static label — never the countdown text itself,
    // which changes every second and would make an unreliable accessible
    // name. `role="timer"` plus this static label is the correct pairing for
    // a live-updating region, so it deliberately departs from the visible
    // text convention used elsewhere in this codebase.
    #[test]
    fn the_dial_exposes_a_stable_timer_role_and_label_for_assistive_tech() {
        let html = render(RootAtDialPartial);
        assert!(
            html.contains(r#"role="timer""#),
            "expected the dial to carry role=\"timer\", got: {html}"
        );
        assert!(
            html.contains("aria-label=\"Minuteur · Lire une page\""),
            "expected a stable accessible name, got: {html}"
        );
    }

    #[test]
    fn the_dial_ring_offset_is_computed_from_remaining_and_total() {
        let html = render(RootAtDialPartial);
        let expected = format!("{}", ring_offset(210, 300));
        assert!(
            html.contains(&expected),
            "expected the ring's stroke-dashoffset to carry the computed value {expected}, got: {html}"
        );
    }

    // @scenario: ritual/S8
    #[test]
    fn the_dial_says_a_gentle_word_only_once_remaining_reaches_zero() {
        let gentle_word = "Vous avez pris ce moment pour vous. C&#39;est déjà beaucoup.";
        assert!(
            render(RootAtDialZero).contains(gentle_word),
            "expected the gentle word at remaining == 0"
        );
        assert!(
            !render(RootAtDialPartial).contains(gentle_word),
            "expected no gentle word while time remains"
        );
    }

    #[test]
    fn the_dial_stops_ticking_once_remaining_reaches_zero() {
        assert!(
            render(RootAtDialPartial).contains("ritual-tick"),
            "expected the tick sensor while time remains"
        );
        assert!(
            !render(RootAtDialZero).contains("ritual-tick"),
            "expected no tick sensor once remaining reaches zero"
        );
    }

    #[test]
    fn firing_the_tick_advances_the_countdown_from_a_live_clock() {
        let mut screen = Screen::open(RootAtActiveHabitGoalFive);
        let before = screen.html();
        assert!(
            before.contains("5:00"),
            "expected the countdown to start at 5:00, got: {before}"
        );

        std::thread::sleep(std::time::Duration::from_millis(1100));
        screen.fire_animation_iteration("animationiteration");

        let after = screen.html();
        assert!(
            !after.contains("5:00"),
            "expected the countdown to have moved off 5:00 after a real tick, got: {after}"
        );
        assert!(
            !after.contains(&format!("stroke-dashoffset=\"{}\"", ring_offset(300, 300))),
            "expected the ring offset to have moved off its starting value, got: {after}"
        );
    }
}
