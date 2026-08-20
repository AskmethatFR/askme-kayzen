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
        }
    }
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

    // Test List — Week screen render (@feature:week-recap, S1-S4 in scope this
    // slice; S5-S7's per-habit row and rhythm dots are task 3/4):
    // - the figure states minutes practised across every habit (S1).
    // - paused and anchored habits still count toward the figure (S2, sum half
    //   only — see get_week_recap.rs's test of the same name for the split).
    // - a fresh week reads its gentle word (S3).
    // - a week without recent practice reads rest, without blame (S4).
    // - the masthead back-link returns to Aujourd'hui.

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
}
