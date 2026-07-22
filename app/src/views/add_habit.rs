use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn AddHabit() -> Element {
    let services = use_context::<Services>();
    let navigator = use_navigator();
    let mut name = use_signal(String::new);

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                Link { class: "quiet-link", to: Route::Today {}, "Aujourd'hui" }
            }
            h1 { class: "greeting", "Ajouter" }
            p { class: "lede", "Une toute petite habitude, à une minute par jour." }

            div { class: "field",
                input {
                    class: "input",
                    value: "{name}",
                    placeholder: "Nom de l'habitude",
                    oninput: move |event| name.set(event.value()),
                }
            }
            button {
                class: "btn btn-primary btn-block",
                onclick: move |_| {
                    if services.add_habit.execute(&name()).is_ok() {
                        navigator.push(Route::Today {});
                    }
                },
                "Ajouter, à 1 min par jour"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::composition::Services;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use std::rc::Rc;

    // The button's onclick runs exactly this action; testing it through the Services
    // registry proves the add path the screen relies on, without a router harness.
    #[test]
    fn submitting_the_form_adds_the_habit_to_today() {
        let services = Services::with_repository(Rc::new(InMemoryHabitRepository::new()));

        services.add_habit.execute("Lire une page").unwrap();

        let titles: Vec<String> = services
            .list_board_habits
            .handle()
            .into_iter()
            .map(|summary| summary.title)
            .collect();
        assert!(titles.contains(&"Lire une page".to_string()));
    }
}
