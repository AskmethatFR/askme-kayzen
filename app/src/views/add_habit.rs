use crate::composition::{STARTING_GOAL, Services};
use crate::i18n::{tr, tr_key};
use crate::route::Route;
use dioxus::prelude::*;
use rand::seq::SliceRandom;

/// Ready-made tiny habits, all one minute — tap one to add it without typing.
const IDEA_KEYS: [&str; 20] = [
    "idea-drink-water",
    "idea-put-away-an-object",
    "idea-write-a-line",
    "idea-stretch",
    "idea-breathe-deeply",
    "idea-read-a-page",
    "idea-walk-a-minute",
    "idea-make-the-bed",
    "idea-note-a-gratitude",
    "idea-close-your-eyes-a-minute",
    "idea-water-a-plant",
    "idea-drink-tea",
    "idea-look-out-the-window",
    "idea-stand-tall-a-minute",
    "idea-listen-to-a-song",
    "idea-tidy-your-desk",
    "idea-meditate-a-minute",
    "idea-smile-at-someone",
    "idea-say-thanks",
    "idea-get-some-air",
];

fn two_random_idea_keys() -> Vec<&'static str> {
    IDEA_KEYS
        .choose_multiple(&mut rand::thread_rng(), 2)
        .copied()
        .collect()
}

#[component]
pub fn AddHabit() -> Element {
    let services = use_context::<Services>();
    let navigator = use_navigator();
    let mut name = use_signal(String::new);
    let idea_keys = use_signal(two_random_idea_keys);
    let name_label = tr!("add-habit-name-input-label");

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                Link { class: "quiet-link", to: Route::Today {}, {tr!("masthead-back-to-today")} }
            }
            h1 { class: "greeting", {tr!("add-habit-heading")} }
            p { class: "lede", {tr!("add-habit-lede")} }

            p { class: "eyebrow", {tr!("add-habit-ideas-eyebrow")} }
            ul { class: "habit-list",
                for key in idea_keys() {
                    {
                        let label = tr_key(key);
                        rsx! {
                            li { key: "{key}", class: "habit-row",
                                div { class: "habit-body",
                                    div { class: "idea-name", "{label}" }
                                    div { class: "habit-meta", {tr!("add-habit-idea-meta", minutes: STARTING_GOAL as i64)} }
                                }
                                button {
                                    class: "add-target",
                                    aria_label: tr!("add-habit-idea-add-aria", label: label.clone()),
                                    onclick: {
                                        let services = services.clone();
                                        let label = label.clone();
                                        move |_| {
                                            if services
                                                .add_habit
                                                .execute(label.clone(), STARTING_GOAL)
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
                }
            }

            p { class: "eyebrow", {tr!("add-habit-own-eyebrow")} }
            div { class: "field",
                input {
                    class: "input",
                    "aria-label": "{name_label}",
                    value: "{name}",
                    placeholder: "{name_label}",
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
                {tr!("add-habit-submit", minutes: STARTING_GOAL as i64)}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IDEA_KEYS, two_random_idea_keys};
    use crate::composition::{STARTING_GOAL, Services};
    use crate::i18n::use_locale_for_tests;
    use crate::route::Route;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use dioxus::prelude::*;
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
    fn two_random_idea_keys_are_two_distinct_keys_from_the_list() {
        let picks = two_random_idea_keys();

        assert_eq!(picks.len(), 2);
        assert_ne!(picks[0], picks[1]);
        assert!(picks.iter().all(|key| IDEA_KEYS.contains(key)));
    }

    #[test]
    fn every_idea_key_resolves_in_both_catalogues() {
        let (fr_ids, en_ids) = crate::i18n::catalogue_ids();

        for key in IDEA_KEYS {
            assert!(fr_ids.contains(key), "expected {key} to resolve in fr.ftl");
            assert!(en_ids.contains(key), "expected {key} to resolve in en.ftl");
        }
    }

    #[component]
    fn RootAtAddHabitScreen() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/add")));
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

    // @scenario: add-habit/S1
    #[test]
    fn opening_add_habit_offers_two_ready_made_ideas_and_a_way_to_add_your_own() {
        let html = render(RootAtAddHabitScreen);

        assert!(
            html.contains("Une nouvelle petite habitude"),
            "expected the gentle invitation heading, got: {html}"
        );
        assert!(
            html.contains("Quelques idées déjà prêtes"),
            "expected the ready-made ideas eyebrow, got: {html}"
        );
        let idea_chip_count = html.matches(r#"class="add-target""#).count();
        assert_eq!(
            idea_chip_count, 2,
            "expected exactly two suggested ideas offered, got: {html}"
        );
        assert!(
            html.contains("Ou la vôtre") && html.contains("Nom de l&#39;habitude"),
            "expected the freeform way to add your own, got: {html}"
        );
        assert!(
            html.contains("Ajouter, à 5 min par jour"),
            "expected the submit gesture named with its own gentle goal, got: {html}"
        );
    }
}
