use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn Today() -> Element {
    let services = use_context::<Services>();
    let habits = services.list_board_habits.handle();

    rsx! {
        h1 { "Bonjour." }
        h4 { "Un seul petit pas suffit pour aujourd'hui." }
        for habit in habits {
            div { style: "display:flex; gap:16px; padding:20px 0;",
                div { flex: "1",
                    Link { to: Route::HabitDetail { id: habit.id.clone() }, "{habit.title}" }
                    div { "aujourd'hui · {habit.minutes} min" }
                }
                span { if habit.done_today { "✓" } else { "○" } }
            }
        }

        div {
            Link { to: Route::AddHabit {}, "Ajouter une toute petite habitude" }
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
    use std::rc::Rc;

    fn seeded_services() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let habit = Habit::new(
            HabitId::new("test-1".to_string()),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            InitialDuration::new(4).unwrap(),
        );
        repository.save(&habit);
        Services::with_repository(repository)
    }

    #[component]
    fn TestRoot() -> Element {
        use_context_provider(seeded_services);
        rsx! {
            Router::<Route> {}
        }
    }

    #[test]
    fn today_renders_board_habits_through_the_wiring() {
        let mut vdom = VirtualDom::new(TestRoot);
        vdom.rebuild_in_place();

        let html = dioxus_ssr::render(&vdom);

        assert!(
            html.contains("Read one page"),
            "expected the seeded habit title, got: {html}"
        );
        assert!(html.contains("4 min"), "expected the dose, got: {html}");
    }
}
