use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;
use kayzen_core::habit_management::queries::get_habit_detail::HabitDetail as HabitDetailData;
use kayzen_core::habit_management::queries::get_habit_detail::HabitState;
use kayzen_core::habit_management::queries::get_habit_detail::RecapMessage;

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
            let staircase = rsx! {
                div {
                    class: "staircase",
                    "aria-label": "Vos sept derniers jours, objectif actuel {habit.current_goal} minutes",
                    for day in habit.days.iter() {
                        span {
                            class: if day.done { "day-bar is-done" } else { "day-bar" },
                            style: "--day-minutes: {day.goal}",
                        }
                    }
                }
            };

            let recap = {
                let days_done_label = plural(habit.recap.days_done, "réalisé", "réalisés");
                let empty_days_label = plural(habit.recap.empty_days, "autre jour", "autres jours");
                let minutes_label = plural(
                    habit.recap.minutes_practised as usize,
                    "minute de pratique accumulée",
                    "minutes de pratique accumulées",
                );

                rsx! {
                    section { class: "recap",
                        p { class: "eyebrow", "Votre histoire" }
                        ul { class: "recap-figures",
                            li {
                                span { class: "recap-figure", "{habit.recap.days_done}" }
                                span { class: "recap-label", "{days_done_label}" }
                            }
                            li {
                                span { class: "recap-figure", "{habit.recap.empty_days}" }
                                span { class: "recap-label", "{empty_days_label}" }
                            }
                            li {
                                span { class: "recap-figure", "{habit.recap.minutes_practised}" }
                                span { class: "recap-label", "{minutes_label}" }
                            }
                            li {
                                span { class: "recap-figure", "{habit.recap.growths}" }
                                span { class: "recap-label", "fois grandie" }
                            }
                            li {
                                span { class: "recap-figure", "{habit.recap.lightenings}" }
                                span { class: "recap-label", "fois allégée" }
                            }
                        }
                        p { class: "quiet-note", "{recap_copy(habit.recap.message)}" }
                    }
                }
            };

            match habit.state {
                HabitState::Active => rsx! {
                    div { class: "screen",
                        header { class: "masthead",
                            Link { class: "quiet-link", to: Route::Today {}, "← Aujourd'hui" }
                        }
                        h1 { class: "greeting", "{habit.title}" }
                        p { class: "lede", "chaque jour · {habit.current_goal} min" }

                        {staircase}

                        {recap}

                        p { class: "eyebrow", "Ajuster, à votre rythme" }
                        button {
                            class: "btn btn-block",
                            aria_label: "Passer à {habit.next_goal_up} min · {habit.title}",
                            onclick: {
                                let services = services.clone();
                                let id = id.clone();
                                move |_| detail.set(grow_and_reload(&services, &id))
                            },
                            "Passer à {habit.next_goal_up} min"
                        }
                        button {
                            class: "btn btn-block",
                            aria_label: "Alléger à {habit.next_goal_down} min · {habit.title}",
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
                            "Commencer ma pratique"
                        }

                        button {
                            class: "btn btn-block",
                            aria_label: "Mettre en pause, sans culpabilité · {habit.title}",
                            onclick: {
                                let services = services.clone();
                                let id = id.clone();
                                move |_| detail.set(pause_and_reload(&services, &id))
                            },
                            "Mettre en pause, sans culpabilité"
                        }

                        button {
                            class: "btn btn-block",
                            aria_label: "L'ancrer · elle est devenue naturelle · {habit.title}",
                            onclick: {
                                let services = services.clone();
                                let id = id.clone();
                                move |_| detail.set(anchor_and_reload(&services, &id))
                            },
                            "L'ancrer · elle est devenue naturelle"
                        }
                    }
                },
                HabitState::Paused => rsx! {
                    div { class: "screen",
                        header { class: "masthead",
                            Link { class: "quiet-link", to: Route::Today {}, "← Aujourd'hui" }
                        }
                        h1 { class: "greeting", "{habit.title}" }
                        p { class: "lede", "en pause · {habit.current_goal} min" }

                        {staircase}

                        {recap}

                        button {
                            class: "btn btn-primary btn-block",
                            aria_label: "La reprendre · {habit.title}",
                            onclick: {
                                let services = services.clone();
                                let id = id.clone();
                                move |_| detail.set(resume_and_reload(&services, &id))
                            },
                            "La reprendre"
                        }
                    }
                },
                HabitState::Anchored => rsx! {
                    div { class: "screen",
                        header { class: "masthead",
                            Link { class: "quiet-link", to: Route::Today {}, "← Aujourd'hui" }
                        }
                        h1 { class: "greeting", "{habit.title}" }
                        p { class: "lede", "ancrée · {habit.current_goal} min" }

                        {staircase}

                        {recap}
                    }
                },
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

#[must_use]
fn pause_and_reload(services: &Services, id: &str) -> Option<HabitDetailData> {
    services.pause_habit.execute(id).ok();
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

#[must_use]
fn recap_copy(message: RecapMessage) -> &'static str {
    match message {
        RecapMessage::FreshStart => "Un début parfait. Tout est encore devant.",
        RecapMessage::Resting => "Elle se repose en ce moment. Elle vous attend, sans presser.",
        RecapMessage::Growing => "Vous la faites vivre, à votre rythme.",
    }
}

#[must_use]
fn plural(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count > 1 { many } else { one }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::click_harness::Screen;
    use dioxus::history::{MemoryHistory, provide_history_context};
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

    // A habit at the floor, done today: the recap reads today's completion, so
    // this fixture pins the singular form on the recap (slice 8).
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

    fn services_with_a_habit_thirty_days_old_done_twelve_days() -> Services {
        let today = LocalDate::from_epoch_day(20_000);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(19_971),
        );
        for days_back in 0..12 {
            habit.toggle_done(LocalDate::from_epoch_day(19_971 + days_back));
        }
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtThirtyDayHabitDoneTwelve() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_thirty_days_old_done_twelve_days);
        rsx! {
            Router::<Route> {}
        }
    }

    fn services_with_a_habit_grown_three_times_and_lightened_once() -> Services {
        let today = LocalDate::from_epoch_day(20_005);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        for _ in 0..3 {
            habit.grow(today);
        }
        habit.lighten(today);
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    fn services_with_a_habit_done_twice_at_five_then_once_at_six() -> Services {
        let today = LocalDate::from_epoch_day(20_005);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        habit.toggle_done(LocalDate::from_epoch_day(20_003));
        habit.toggle_done(LocalDate::from_epoch_day(20_004));
        habit.grow(today);
        habit.toggle_done(today);
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    fn services_with_a_habit_resting_for_ten_days() -> Services {
        let today = LocalDate::from_epoch_day(20_020);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        habit.toggle_done(today.minus_days(10));
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtHabitRestingForTenDays() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_resting_for_ten_days);
        rsx! {
            Router::<Route> {}
        }
    }

    fn services_with_a_brand_new_habit() -> Services {
        let today = LocalDate::from_epoch_day(20_000);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            today,
        ));
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtBrandNewHabit() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_brand_new_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtHabitDoneTwiceAtFiveThenOnceAtSix() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_done_twice_at_five_then_once_at_six);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtHabitGrownThreeTimesLightenedOnce() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_grown_three_times_and_lightened_once);
        rsx! {
            Router::<Route> {}
        }
    }

    // Five pairwise-distinct values, one per HabitRecap field — mirrors
    // get_habit_detail's field-integrity fixture, one layer up.
    fn services_with_a_habit_with_five_pairwise_distinct_recap_figures() -> Services {
        let today = LocalDate::from_epoch_day(20_006);
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(today));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = Habit::new(
            HabitId::new("h-1").unwrap(),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(10).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        habit.grow(LocalDate::from_epoch_day(20_001));
        habit.grow(LocalDate::from_epoch_day(20_002));
        habit.lighten(LocalDate::from_epoch_day(20_003));
        habit.toggle_done(LocalDate::from_epoch_day(20_000));
        habit.toggle_done(LocalDate::from_epoch_day(20_001));
        habit.toggle_done(LocalDate::from_epoch_day(20_004));
        habit.toggle_done(today);
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    #[component]
    fn RootAtHabitWithFivePairwiseDistinctRecapFigures() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_a_habit_with_five_pairwise_distinct_recap_figures);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtPausedHabit() -> Element {
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

    // Asserts the figure and its label sit adjacent, as the SAME field: a
    // `contains(number) && contains(word)` pair passes even when the numbers
    // and words belong to two DIFFERENT rows swapped with each other. This
    // checks the actual rendered adjacency, so a field swap fails it.
    fn figure_pair(html: &str, figure: impl std::fmt::Display, label: &str) -> bool {
        html.contains(&format!(
            "<span class=\"recap-figure\">{figure}</span><span class=\"recap-label\">{label}</span>"
        ))
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
            html.matches("day-bar").count(),
            7,
            "expected the practice staircase to stay on a paused habit, got: {html}"
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
            html.matches("day-bar").count(),
            7,
            "expected the practice staircase to stay on an anchored habit, got: {html}"
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
            !html.contains("day-bar"),
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

    // @scenario: habit-stats/S1
    #[test]
    fn the_recap_names_the_days_without_practice_and_never_a_failure() {
        let html = render(RootAtThirtyDayHabitDoneTwelve);

        assert!(
            figure_pair(&html, 12, "réalisés"),
            "expected 12 days done to be shown, got: {html}"
        );
        assert!(
            figure_pair(&html, 18, "autres jours"),
            "expected the 18 days without practice to be shown, got: {html}"
        );
        let lowercase_html = html.to_lowercase();
        for forbidden in [
            "échec", "raté", "manqué", "perdu", "oublié", "failed", "vide",
        ] {
            assert!(
                !lowercase_html.contains(forbidden),
                "expected no failure word in the recap, got: {html}"
            );
        }
    }

    #[test]
    fn the_recap_is_shown_whatever_the_habits_state() {
        for root in [RootAtKnownHabit, RootAtPausedHabit, RootAtAnchoredHabit] {
            let html = render(root);

            assert!(
                html.contains("class=\"recap\""),
                "expected the recap zone on every habit state, got: {html}"
            );
        }
    }

    #[test]
    fn a_single_day_reads_in_the_singular() {
        let html = render(RootAtFloorHabitDoneToday);

        assert!(
            figure_pair(&html, 1, "réalisé"),
            "expected the singular form for a single day done, got: {html}"
        );
        assert!(
            !figure_pair(&html, 1, "réalisés"),
            "expected no plural form for a single day done, got: {html}"
        );
    }

    // @scenario: habit-stats/S2
    #[test]
    fn the_recap_shows_how_often_the_goal_moved() {
        let html = render(RootAtHabitGrownThreeTimesLightenedOnce);

        assert!(
            figure_pair(&html, 3, "fois grandie"),
            "expected three growths to be shown, got: {html}"
        );
        assert!(
            figure_pair(&html, 1, "fois allégée"),
            "expected one lightening to be shown, got: {html}"
        );
        assert_eq!(
            html.matches("class=\"recap-figure\"").count(),
            5,
            "expected no sixth recap row for the current goal, got: {html}"
        );
    }

    // @scenario: habit-stats/S3
    #[test]
    fn the_recap_shows_the_minutes_practised() {
        let html = render(RootAtHabitDoneTwiceAtFiveThenOnceAtSix);

        assert!(
            figure_pair(&html, 16, "minutes de pratique accumulées"),
            "expected the sixteen practised minutes to be shown, got: {html}"
        );
    }

    // No Gherkin scenario names this field-by-field integrity (mirrors
    // get_habit_detail's `the_recap_carries_each_figure_in_its_own_field`, one
    // layer up: cargo-mutants never mutates rsx! markup, and a
    // `contains(number) && contains(word)` pair passes on a swapped field just
    // as readily as on the right one — this is exactly how B1 shipped green).
    // Five pairwise-distinct figures make any swap observable.
    #[test]
    fn the_recap_view_carries_each_figure_in_its_own_field() {
        let html = render(RootAtHabitWithFivePairwiseDistinctRecapFigures);

        assert!(
            figure_pair(&html, 4, "réalisés"),
            "expected days_done in its own field, got: {html}"
        );
        assert!(
            figure_pair(&html, 3, "autres jours"),
            "expected empty_days in its own field, got: {html}"
        );
        assert!(
            figure_pair(&html, 43, "minutes de pratique accumulées"),
            "expected minutes_practised in its own field, got: {html}"
        );
        assert!(
            figure_pair(&html, 2, "fois grandie"),
            "expected growths in its own field, got: {html}"
        );
        assert!(
            figure_pair(&html, 1, "fois allégée"),
            "expected lightenings in its own field, got: {html}"
        );
    }

    // @scenario: habit-stats/S4 — the fixture practises the habit once before
    // the 10 empty days (N3): the bare "no completion for the last 10 days"
    // Given also fit a never-practised habit, which reads FreshStart, not
    // this resting sentence.
    #[test]
    fn a_resting_habits_recap_acknowledges_the_rest_without_blaming() {
        let html = render(RootAtHabitRestingForTenDays);

        assert!(
            html.contains("Elle se repose en ce moment"),
            "expected the resting sentence to be shown, got: {html}"
        );
        let lowercase_html = html.to_lowercase();
        for forbidden in [
            "échec", "raté", "manqué", "perdu", "oublié", "failed", "vide",
        ] {
            assert!(
                !lowercase_html.contains(forbidden),
                "expected no failure word in a resting recap, got: {html}"
            );
        }
    }

    // @scenario: habit-stats/S5
    #[test]
    fn a_brand_new_habits_recap_opens_on_a_perfect_start() {
        let html = render(RootAtBrandNewHabit);

        assert!(
            html.contains("Un début parfait"),
            "expected the fresh-start sentence to be shown, got: {html}"
        );
    }

    // plural() is app-crate code, excluded from the mutation scope
    // (.cargo/mutants.toml) — this branch was invisible to both gates until
    // this assertion. Zero reads as singular in French ("0 réalisé", never
    // "0 réalisés").
    #[test]
    fn zero_reads_in_the_singular() {
        let html = render(RootAtBrandNewHabit);

        assert!(
            figure_pair(&html, 0, "réalisé"),
            "expected the singular form at zero, got: {html}"
        );
        assert!(
            !figure_pair(&html, 0, "réalisés"),
            "expected no plural form at zero, got: {html}"
        );
    }
}
