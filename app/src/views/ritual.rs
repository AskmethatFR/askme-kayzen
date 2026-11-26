use crate::composition::Services;
use crate::i18n::tr;
use crate::route::Route;
use dioxus::prelude::*;
use dioxus::router::Navigator;
use kayzen_core::habit_management::queries::get_habit_detail::HabitState;

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
                PracticeCounter {
                    id: habit.id.clone(),
                    title: habit.title.clone(),
                }
            },
        },
    }
}

#[component]
fn PracticeCounter(id: String, title: String) -> Element {
    let services = use_context::<Services>();
    let navigator = use_navigator();

    rsx! {
            div { class: "screen ritual",
                h1 { class: "greeting", "{title}" }
                h3
                {
                    class: "motivation", {tr!("ritual-gentle-word")}
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

fn complete_and_go_home(services: &Services, navigator: Navigator, id: &str) {
    record_todays_practice(services, id);
    navigator.push(Route::Today {});
}

fn record_todays_practice(services: &Services, id: &str) {
    services.mark_done.execute(id).ok();
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
    fn RootAtActiveHabitGoalOne() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path(
                "/habit/h-1/ritual",
            )));
        });
        use_context_provider(|| services_with_an_active_habit(1));
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
    fn the_ritual_heading_carries_the_habits_name_alone() {
        let html = render(RootAtActiveHabitGoalFive);

        assert!(
            html.contains(r#"<h1 class="greeting">Lire une page</h1>"#),
            "expected the habit's name alone as the heading, got: {html}"
        );
    }

    // @scenario: ritual/S1
    #[test]
    fn opening_the_ritual_shows_the_habit_as_a_counter_with_no_duration() {
        let html = render(RootAtActiveHabitGoalFive);

        for absent in [" min", "5:00", "ritual-dial", "ritual-countdown"] {
            assert!(
                !html.contains(absent),
                "expected no duration on the ritual (found {absent}), got: {html}"
            );
        }
        assert!(
            html.contains(r#"aria-label="J&#39;ai terminé · Lire une page""#),
            "expected the completion gesture, got: {html}"
        );
    }

    // @scenario: ritual/S3
    #[test]
    fn stopping_the_ritual_returns_to_the_detail_screen_recording_nothing() {
        let mut screen = Screen::open(RootAtActiveHabitGoalFive);

        screen.click("Arrêter, ce n'est pas grave · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("Commencer"),
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
        let mut screen = Screen::open(RootAtActiveHabitGoalOne);

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
    fn the_completion_gesture_adds_one_attempt_on_top_of_a_day_that_already_reached_the_goal() {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit_with_goal(5));
        SHARED_REPOSITORY.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&repository)));
        let _guard = SharedRepositoryGuard;
        let mut screen = Screen::open(RootAtActiveHabitOverSharedRepository);

        // Mutated AFTER mount (the component already ran `use_hook`), BEFORE
        // the click — a mount-time snapshot cannot see this, only a click-time
        // read can.
        let mut habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        for _ in 0..5 {
            habit.record_attempt(LocalDate::from_epoch_day(20_005));
        }
        repository.save(&habit);
        screen.click("J'ai terminé · Lire une page");

        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(
            habit.attempts_on(LocalDate::from_epoch_day(20_005)),
            6,
            "expected the completion to count one more attempt, past the goal of five"
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
        assert_eq!(
            habit.attempts_on(LocalDate::from_epoch_day(20_005)),
            2,
            "expected each completion to count its own attempt"
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
}
