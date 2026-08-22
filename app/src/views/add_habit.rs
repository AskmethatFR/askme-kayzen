use crate::composition::{STARTING_GOAL, Services};
use crate::route::Route;
use dioxus::prelude::*;
use rand::seq::SliceRandom;

/// Ready-made tiny habits, all one minute — tap one to add it without typing.
const IDEAS: [&str; 20] = [
    "Boire un verre d'eau",
    "Ranger un objet",
    "Écrire une ligne",
    "S'étirer",
    "Respirer profondément",
    "Lire une page",
    "Marcher une minute",
    "Faire son lit",
    "Noter une gratitude",
    "Fermer les yeux une minute",
    "Arroser une plante",
    "Boire un thé",
    "Regarder par la fenêtre",
    "Se tenir droit une minute",
    "Écouter une chanson",
    "Ranger son bureau",
    "Méditer une minute",
    "Sourire à quelqu'un",
    "Dire merci",
    "Prendre l'air",
];

fn two_random_ideas() -> Vec<&'static str> {
    IDEAS
        .choose_multiple(&mut rand::thread_rng(), 2)
        .copied()
        .collect()
}

#[component]
pub fn AddHabit() -> Element {
    let services = use_context::<Services>();
    let navigator = use_navigator();
    let mut name = use_signal(String::new);
    let ideas = use_signal(two_random_ideas);

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                Link { class: "quiet-link", to: Route::Today {}, "← Aujourd'hui" }
            }
            h1 { class: "greeting", "Une nouvelle petite habitude" }
            p { class: "lede",
                "Un objectif doux : 5 minutes par jour. Moins, c'est déjà quelque chose ; plus, tant mieux."
            }

            p { class: "eyebrow", "Quelques idées déjà prêtes" }
            ul { class: "habit-list",
                for idea in ideas() {
                    li { key: "{idea}", class: "habit-row",
                        div { class: "habit-body",
                            div { class: "idea-name", "{idea}" }
                            div { class: "habit-meta", "5 min par jour" }
                        }
                        button {
                            class: "add-target",
                            aria_label: "Ajouter « {idea} »",
                            onclick: {
                                let services = services.clone();
                                move |_| {
                                    if services
                                        .add_habit
                                        .execute(idea.to_string(), STARTING_GOAL)
                                        .is_ok()
                                    {
                                        navigator.push(Route::Today {});
                                    }
                                }
                            },
                            "+"
                        }
                    }
                }
            }

            p { class: "eyebrow", "Ou la vôtre" }
            div { class: "field",
                input {
                    class: "input",
                    "aria-label": "Nom de l'habitude",
                    value: "{name}",
                    placeholder: "Nom de l'habitude",
                    oninput: move |event| name.set(event.value()),
                }
            }
            button {
                class: "btn btn-primary btn-block",
                onclick: {
                    let services = services.clone();
                    move |_| {
                        if services.add_habit.execute(name(), STARTING_GOAL).is_ok() {
                            navigator.push(Route::Today {});
                        }
                    }
                },
                "Ajouter, à 5 min par jour"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IDEAS, two_random_ideas};
    use crate::composition::{STARTING_GOAL, Services};
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use std::rc::Rc;

    // The submit button and each idea chip run this action; testing it through the
    // Services registry proves the add path the screen relies on, without a router.
    #[test]
    fn adding_a_habit_makes_it_appear_on_today() {
        let services = Services::with_repository(Rc::new(InMemoryHabitRepository::new()));

        services
            .add_habit
            .execute("Lire une page".to_string(), STARTING_GOAL)
            .unwrap();

        let titles: Vec<String> = services
            .list_board_habits
            .handle()
            .active
            .into_iter()
            .map(|summary| summary.title)
            .collect();
        assert!(titles.contains(&"Lire une page".to_string()));
    }

    #[test]
    fn two_random_ideas_are_two_distinct_ideas_from_the_list() {
        let picks = two_random_ideas();

        assert_eq!(picks.len(), 2);
        assert_ne!(picks[0], picks[1]);
        assert!(picks.iter().all(|idea| IDEAS.contains(idea)));
    }
}
