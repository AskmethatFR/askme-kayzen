use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn Today() -> Element {
    let services = use_context::<Services>();
    let mut habits = use_signal({
        let services = services.clone();
        move || services.list_board_habits.handle()
    });

    let list = habits();
    let total = list.len();
    let done = list.iter().filter(|habit| habit.done_today).count();

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
                for habit in list {
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

#[cfg(test)]
mod tests {
    use crate::composition::Services;
    use crate::route::Route;
    use dioxus::prelude::*;
    use kayzen_core::habit_management::domain::habit::Habit;
    use kayzen_core::habit_management::domain::habit_id::HabitId;
    use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
    use kayzen_core::habit_management::domain::habit_title::HabitTitle;
    use kayzen_core::habit_management::domain::initial_duration::InitialDuration;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use kayzen_core::shared::clock::{Clock, SystemClock};
    use std::rc::Rc;

    fn a_habit() -> Habit {
        Habit::new(
            HabitId::new("test-1".to_string()),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            InitialDuration::new(4).unwrap(),
        )
    }

    fn services_with_one_undone_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        Services::with_repository(repository)
    }

    fn services_with_one_habit_done_today() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.toggle_done(SystemClock.today());
        repository.save(&habit);
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
}
