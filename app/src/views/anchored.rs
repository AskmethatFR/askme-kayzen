use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn Anchored() -> Element {
    let services = use_context::<Services>();
    let habits = services.list_anchored_habits.handle();
    let count = habits.len();

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                Link { class: "quiet-link", to: Route::Today {}, "← Aujourd'hui" }
            }
            h1 { class: "greeting", "Ancrées" }
            ul { class: "habit-list",
                for habit in habits {
                    li { class: "habit-row",
                        span { class: "habit-name", "{habit.title}" }
                    }
                }
            }
            p { class: "tally", "{count} · devenues naturelles" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::Services;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use kayzen_core::habit_management::domain::goal::Goal;
    use kayzen_core::habit_management::domain::habit::Habit;
    use kayzen_core::habit_management::domain::habit_id::HabitId;
    use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
    use kayzen_core::habit_management::domain::habit_title::HabitTitle;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use kayzen_core::shared::local_date::LocalDate;
    use std::rc::Rc;

    fn a_habit(id: &str, title: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new(title.to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        )
    }

    fn services_with_two_anchored_habits() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut first = a_habit("h-1", "Lire une page");
        first.anchor();
        repository.save(&first);
        let mut second = a_habit("h-2", "Bouger un peu");
        second.anchor();
        repository.save(&second);
        Services::with_repository(repository)
    }

    #[component]
    fn RootAtAnchoredScreen() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/anchored")));
        });
        use_context_provider(services_with_two_anchored_habits);
        rsx! {
            Router::<Route> {}
        }
    }

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    // @scenario: anchor-habit/S2
    #[test]
    fn the_ancrees_screen_lists_each_title_and_states_the_count() {
        let html = render(RootAtAnchoredScreen);

        assert!(
            html.contains("Lire une page") && html.contains("Bouger un peu"),
            "expected both anchored habits' titles, got: {html}"
        );
        assert!(
            html.contains("2 · devenues naturelles"),
            "expected the count line's full copy naming how many are anchored, got: {html}"
        );
    }
}
