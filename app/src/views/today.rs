use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;
use kayzen_core::habit_management::queries::list_board_habits::TodayHabits;

#[component]
pub fn Today() -> Element {
    let services = use_context::<Services>();
    let mut habits = use_signal({
        let services = services.clone();
        move || services.list_board_habits.handle()
    });

    let today_habits = habits();
    let total = today_habits.active.len();
    let done = today_habits
        .active
        .iter()
        .filter(|habit| habit.done_today)
        .count();

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                span { class: "masthead-date", "Aujourd'hui" }
                span { class: "tag tag-accent", "Kaizen" }
            }
            h1 { class: "greeting", "Bonjour." }
            p { class: "lede", "Un seul petit pas suffit pour aujourd'hui." }

            p { class: "eyebrow", "Vos petits pas" }
            ul { class: "habit-list",
                for habit in today_habits.active {
                    li { class: "habit-row",
                        div { class: "habit-body",
                            Link {
                                class: "habit-name",
                                to: Route::HabitDetail { id: habit.id.clone() },
                                "{habit.title}"
                            }
                            div { class: "habit-meta", "chaque jour · {habit.minutes} min" }
                        }
                        button {
                            class: if habit.done_today { "target is-done" } else { "target" },
                            aria_label: if habit.done_today { "Fait aujourd'hui" } else { "Marquer comme fait" },
                            onclick: {
                                let services = services.clone();
                                let id = habit.id.clone();
                                move |_| {
                                    services.mark_done.execute(&id).ok();
                                    habits.set(services.list_board_habits.handle());
                                }
                            },
                            span { class: "target-ink" }
                        }
                    }
                }
            }

            if !today_habits.paused.is_empty() {
                p { class: "eyebrow", "En pause · aucune pression" }
                ul { class: "habit-list",
                    for habit in today_habits.paused {
                        li { class: "habit-row is-paused",
                            div { class: "habit-body",
                                Link {
                                    class: "habit-name",
                                    to: Route::HabitDetail { id: habit.id.clone() },
                                    "{habit.title}"
                                }
                            }
                            button {
                                class: "resume-link",
                                onclick: {
                                    let services = services.clone();
                                    let id = habit.id.clone();
                                    move |_| habits.set(resume_and_relist(&services, &id))
                                },
                                "Reprendre"
                            }
                        }
                    }
                }
            }

            p { class: "tally", "{done} sur {total} · c'est déjà quelque chose." }
            Link {
                class: "quiet-link",
                to: Route::Week {},
                "Voir comment je grandis · cette semaine"
            }
            div { class: "add-cta",
                Link {
                    class: "quiet-link",
                    to: Route::AddHabit {},
                    "+ Ajouter une toute petite habitude"
                }
            }
        }
    }
}

#[must_use]
fn resume_and_relist(services: &Services, id: &str) -> TodayHabits {
    services.resume_habit.execute(id).ok();
    services.list_board_habits.handle()
}

#[cfg(test)]
mod tests {
    use crate::composition::Services;
    use crate::route::Route;
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

    struct FixedClock(LocalDate);

    impl Clock for FixedClock {
        fn today(&self) -> LocalDate {
            self.0
        }
    }

    fn a_habit() -> Habit {
        Habit::new(
            HabitId::new("test-1").unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(4).unwrap(),
            LocalDate::from_epoch_day(20_000),
        )
    }

    fn services_with_one_undone_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        Services::with_repository(repository)
    }

    fn services_with_one_habit_done_today() -> Services {
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_005)));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.toggle_done(clock.today());
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    fn services_with_one_active_and_one_paused_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        let mut paused = Habit::new(
            HabitId::new("test-2").unwrap(),
            HabitTitle::new("Move a little".to_string()).unwrap(),
            Goal::new(2).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        paused.pause();
        repository.save(&paused);
        Services::with_repository(repository)
    }

    #[component]
    fn RootWithUndoneHabit() -> Element {
        use_context_provider(services_with_one_undone_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithHabitDoneToday() -> Element {
        use_context_provider(services_with_one_habit_done_today);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithActiveAndPausedHabit() -> Element {
        use_context_provider(services_with_one_active_and_one_paused_habit);
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
    fn today_renders_board_habits_through_the_wiring() {
        let html = render(RootWithUndoneHabit);

        assert!(
            html.contains("Read one page"),
            "expected the seeded habit title, got: {html}"
        );
        assert!(html.contains("4 min"), "expected the dose, got: {html}");
    }

    #[test]
    fn a_habit_done_today_renders_as_stamped() {
        let html = render(RootWithHabitDoneToday);

        assert!(
            html.contains("target is-done"),
            "expected the done target to be stamped, got: {html}"
        );
    }

    // @scenario: pause-resume/S1
    #[test]
    fn a_paused_habit_renders_under_the_paused_eyebrow_and_the_tally_counts_only_active() {
        let html = render(RootWithActiveAndPausedHabit);

        assert!(
            html.contains("En pause") && html.contains("aucune pression"),
            "expected the paused-zone eyebrow, got: {html}"
        );
        assert!(
            html.contains("Move a little"),
            "expected the paused habit's title to render, got: {html}"
        );
        assert!(
            html.contains("sur 1 ·"),
            "expected the tally's total to count the active habit only, not the paused one, got: {html}"
        );
    }

    // @scenario: pause-resume/S2
    #[test]
    fn a_paused_row_carries_its_resume_affordance() {
        let html = render(RootWithActiveAndPausedHabit);

        assert!(
            html.contains("Reprendre"),
            "expected the paused row to offer a one-tap resume gesture, got: {html}"
        );
    }

    // @scenario: pause-resume/S2
    #[test]
    fn resume_and_relist_resumes_the_habit_and_returns_the_refreshed_board() {
        let services = services_with_one_active_and_one_paused_habit();

        let board = super::resume_and_relist(&services, "test-2");

        assert!(
            board.active.iter().any(|habit| habit.id == "test-2"),
            "expected the resumed habit to reappear in active, got: {board:?}"
        );
        assert!(
            !board.paused.iter().any(|habit| habit.id == "test-2"),
            "expected the resumed habit to leave the paused zone, got: {board:?}"
        );
    }
}
