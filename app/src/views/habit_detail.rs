use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;
use kayzen_core::habit_management::queries::get_habit_detail::HabitDetail as HabitDetailData;

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
            let step_count = habit.steps.len();
            rsx! {
                div { class: "screen",
                    header { class: "masthead",
                        Link { class: "quiet-link", to: Route::Today {}, "← Aujourd'hui" }
                    }
                    h1 { class: "greeting", "{habit.title}" }
                    p { class: "lede", "chaque jour · {habit.current_goal} min" }

                    div {
                        class: "staircase",
                        "aria-label": "Escalier de progression, objectif actuel {habit.current_goal} minutes",
                        for (index , minutes) in habit.steps.iter().enumerate() {
                            span {
                                class: if index + 1 == step_count { "step-bar is-current" } else { "step-bar" },
                                style: "--step-minutes: {minutes}",
                            }
                        }
                    }

                    p { class: "eyebrow", "Ajuster, à votre rythme" }
                    button {
                        class: "btn btn-block",
                        onclick: {
                            let services = services.clone();
                            let id = id.clone();
                            move |_| detail.set(grow_and_reload(&services, &id))
                        },
                        "Passer à {habit.next_goal_up} min"
                    }
                    button {
                        class: "btn btn-block",
                        onclick: {
                            let services = services.clone();
                            let id = id.clone();
                            move |_| detail.set(lighten_and_reload(&services, &id))
                        },
                        "Alléger à {habit.next_goal_down} min"
                    }

                    Link {
                        class: "btn btn-primary btn-block",
                        to: Route::Ritual { id: habit.id.clone() },
                        "Faire ma minute"
                    }
                }
            }
        }
        None => rsx! {
            div { class: "screen",
                p { class: "lede", "Cette habitude n'est plus sur votre liste." }
                Link { class: "quiet-link", to: Route::Today {}, "Retour à Aujourd'hui" }
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

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use kayzen_core::habit_management::domain::goal::Goal;
    use kayzen_core::habit_management::domain::habit::Habit;
    use kayzen_core::habit_management::domain::habit_id::HabitId;
    use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
    use kayzen_core::habit_management::domain::habit_title::HabitTitle;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
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

    fn services_with_one_habit_grown_once() -> Services {
        let services = services_with_one_habit();
        services.grow_goal.execute("h-1").ok();
        services
    }

    // Mirrors S4's Given verbatim ("whatever its completions and its current
    // goal"): today's completion stays inert until HabitDetail carries it, so
    // no mutation here can currently make it matter — kept deliberately for
    // when it does, not an oversight.
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

    #[component]
    fn RootAtKnownHabit() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtUnknownHabit() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/missing")));
        });
        use_context_provider(services_with_no_habits);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtGrownHabit() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_habit_grown_once);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtFloorHabitDoneToday() -> Element {
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
    fn a_known_habit_renders_its_title_goal_and_staircase() {
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
            html.contains("step-bar is-current"),
            "expected the current-step accent on the staircase, got: {html}"
        );
        assert_eq!(
            html.matches("step-bar").count(),
            1,
            "expected exactly one staircase bar for a one-step habit, got: {html}"
        );
        assert!(
            html.contains("Passer à 6 min"),
            "expected the grow-goal button offering the next step up, got: {html}"
        );
    }

    #[test]
    fn a_habit_grown_once_renders_two_bars_with_only_the_last_current() {
        let html = render(RootAtGrownHabit);

        assert_eq!(
            html.matches("step-bar").count(),
            2,
            "expected two staircase bars for a twice-stepped habit, got: {html}"
        );
        assert_eq!(
            html.matches("step-bar is-current").count(),
            1,
            "expected exactly one bar carrying is-current, got: {html}"
        );
        let last_bar = html.rfind("step-bar").expect("at least one staircase bar");
        let current = html.find("is-current").expect("one bar carries is-current");
        assert!(
            current > last_bar,
            "expected is-current on the LAST bar specifically (not merely one bar out of two), got: {html}"
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
    fn the_staircase_draws_one_bar_for_each_day_of_the_window() {
        let html = render(RootAtKnownHabit);

        assert_eq!(
            html.matches("day-bar").count(),
            7,
            "expected one bar per calendar day of the window, got: {html}"
        );
    }

    // @scenario: practice-staircase/S2
    #[test]
    fn only_the_practised_days_are_filled_and_the_rest_stay_faint() {
        let html = render(RootAtFloorHabitDoneToday);

        assert_eq!(
            html.matches("day-bar").count(),
            7,
            "expected the missed days to keep their bar rather than leave a gap, got: {html}"
        );
        assert_eq!(
            html.matches("is-done").count(),
            1,
            "expected only the one practised day filled, got: {html}"
        );
    }

    #[test]
    fn an_unknown_habit_shows_a_quiet_fallback_with_a_link_back() {
        let html = render(RootAtUnknownHabit);

        assert!(
            !html.contains("step-bar"),
            "expected no staircase for a missing habit, got: {html}"
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
}
