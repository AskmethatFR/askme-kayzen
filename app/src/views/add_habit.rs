use crate::composition::{STARTING_GOAL, Services};
use crate::i18n::{tr, tr_key};
use crate::route::Route;
use crate::views::back_link::BackLink;
use dioxus::prelude::*;
use kayzen_core::habit_management::domain::habit::Habit;
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
    let daily_life_full =
        services.list_anchored_habits.handle().in_daily_life >= Habit::MAX_IN_DAILY_LIFE;

    if daily_life_full {
        return rsx! {
            div { class: "screen",
                header { class: "masthead",
                    BackLink {}
                }
                h1 { class: "greeting", {tr!("add-habit-heading")} }
                p { class: "quiet-note", {tr!("add-habit-full-note")} }
            }
        };
    }

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                BackLink {}
            }
            h1 { class: "greeting", {tr!("add-habit-heading")} }
            p { class: "lede", {tr!("add-habit-lede", goal: STARTING_GOAL as i64)} }

            h2 { class: "eyebrow", {tr!("add-habit-ideas-eyebrow")} }
            ul { class: "habit-list",
                for key in idea_keys() {
                    {
                        let label = tr_key(key);
                        rsx! {
                            li { key: "{key}", class: "habit-row",
                                div { class: "habit-body",
                                    div { class: "idea-name", "{label}" }
                                    div { class: "habit-meta", {tr!("add-habit-idea-meta", goal: STARTING_GOAL as i64)} }
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

            h2 { class: "eyebrow", {tr!("add-habit-own-eyebrow")} }
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
                {tr!("add-habit-submit", goal: STARTING_GOAL as i64)}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IDEA_KEYS, tr_key, two_random_idea_keys};
    use crate::composition::{STARTING_GOAL, Services};
    use crate::i18n::{use_locale_for_tests, use_locale_for_tests_as};
    use crate::route::Route;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use dioxus::prelude::*;
    use dioxus_i18n::unic_langid::langid;
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
            html.contains("Ajouter, à 3 fois par jour"),
            "expected the submit gesture named with its own gentle goal, got: {html}"
        );
    }

    fn services_with_habit_added_through_the_add_flow() -> Services {
        let services = Services::with_repository(Rc::new(InMemoryHabitRepository::new()));
        services
            .add_habit
            .execute("Lire une page".to_string(), STARTING_GOAL)
            .unwrap();
        services
    }

    #[component]
    fn RootAtTodayAfterAddingAHabit() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/")));
        });
        use_context_provider(services_with_habit_added_through_the_add_flow);
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: add-habit/S7
    #[test]
    fn a_habit_added_at_the_default_goal_shows_three_times_a_day_on_today() {
        let html = render(RootAtTodayAfterAddingAHabit);

        assert!(
            html.contains("3 fois par jour") && !html.contains("min"),
            "expected the default goal of a new habit as times a day, not minutes, got: {html}"
        );
    }

    fn services_holding_habits(count: usize, paused: usize) -> Services {
        let services = Services::with_repository(Rc::new(InMemoryHabitRepository::new()));
        for index in 0..count {
            services
                .add_habit
                .execute(format!("Habitude {index}"), STARTING_GOAL)
                .unwrap();
        }
        for summary in services
            .list_board_habits
            .handle()
            .active
            .iter()
            .take(paused)
        {
            services.pause_habit.execute(&summary.id).unwrap();
        }
        services
    }

    #[component]
    fn RootAtAddHabitScreenWithFiveHabitsOneOfThemPaused() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/add")));
        });
        use_context_provider(|| services_holding_habits(5, 1));
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtAddHabitScreenWithFourHabits() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/add")));
        });
        use_context_provider(|| services_holding_habits(4, 0));
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtAddHabitScreenWithFiveHabitsInEnglish() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/add")));
        });
        use_context_provider(|| services_holding_habits(5, 0));
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: add-habit/S8
    #[test]
    fn a_full_daily_life_blocks_the_add_screen_gently_and_invites_anchoring_one() {
        let html = render(RootAtAddHabitScreenWithFiveHabitsOneOfThemPaused);

        assert!(
            html.contains(
                r#"<p class="quiet-note">Le quotidien est complet · pour en ajouter une, ancrez-en une d&#39;abord</p>"#
            ),
            "expected the gentle invitation to anchor one first, got: {html}"
        );
        assert!(
            !html.contains(r#"class="add-target""#) && !html.contains("<input"),
            "expected no idea and no field on a full daily life, got: {html}"
        );
        assert!(
            html.contains(r#"class="detail-back""#),
            "expected the way back to stay, got: {html}"
        );
    }

    // @scenario: add-habit/S8
    #[test]
    fn four_habits_in_daily_life_still_offer_the_form_without_the_invitation() {
        let html = render(RootAtAddHabitScreenWithFourHabits);

        assert!(
            html.contains(r#"class="add-target""#) && html.contains("<input"),
            "expected the ideas and the field below the cap, got: {html}"
        );
        assert!(
            !html.contains("Le quotidien est complet"),
            "expected no invitation below the cap, got: {html}"
        );
    }

    // @scenario: add-habit/S8
    #[test]
    fn a_full_daily_life_invites_anchoring_one_in_english() {
        let html = render(RootAtAddHabitScreenWithFiveHabitsInEnglish);

        assert!(
            html.contains(
                r#"<p class="quiet-note">Your daily life is full · to add one, anchor one first</p>"#
            ),
            "expected the invitation in English, got: {html}"
        );
        assert!(
            !html.contains(r#"class="add-target""#) && !html.contains("<input"),
            "expected no idea and no field on a full daily life, got: {html}"
        );
    }

    #[component]
    fn RootAtAddHabitScreenAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/add")));
        });
        use_context_provider(|| Services::with_repository(Rc::new(InMemoryHabitRepository::new())));
        rsx! {
            Router::<Route> {}
        }
    }

    const IDEA_TEXT_EN: [&str; 20] = [
        "Drink a glass of water",
        "Put an object away",
        "Write a line",
        "Stretch",
        "Breathe deeply",
        "Read a page",
        "Walk for a minute",
        "Make your bed",
        "Note a gratitude",
        "Close your eyes for a minute",
        "Water a plant",
        "Drink a tea",
        "Look out the window",
        "Stand tall for a minute",
        "Listen to a song",
        "Tidy your desk",
        "Meditate for a minute",
        "Smile at someone",
        "Say thanks",
        "Get some air",
    ];

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_add_habit_screen_in_english() {
        let html = render(RootAtAddHabitScreenAndEnglishLocale);

        assert!(
            html.contains(r#"<h1 class="greeting">A new tiny habit</h1>"#),
            "expected the heading in English, got: {html}"
        );
        assert!(
            html.contains(r#"<h2 class="eyebrow">A few ideas, ready to go</h2>"#),
            "expected the ready-made ideas eyebrow in English, got: {html}"
        );
        assert!(
            html.contains(r#"<h2 class="eyebrow">Or your own</h2>"#),
            "expected the freeform-own eyebrow in English, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Habit name""#),
            "expected the name-input aria-label in English, got: {html}"
        );
        assert!(
            html.contains(">Add, at 3 times a day<"),
            "expected the submit gesture in English, got: {html}"
        );
    }

    #[component]
    fn RootIdeaKeysInEnglish() -> Element {
        use_locale_for_tests_as(langid!("en"));
        rsx! {
            ul {
                for key in IDEA_KEYS {
                    li { "{tr_key(key)}" }
                }
            }
        }
    }

    // @scenario: language/S1
    #[test]
    fn every_idea_key_translates_to_its_pinned_english_text() {
        let html = render(RootIdeaKeysInEnglish);

        for (key, expected) in IDEA_KEYS.iter().zip(IDEA_TEXT_EN.iter()) {
            assert!(
                html.contains(&format!("<li>{expected}</li>")),
                "expected {key} to translate to {expected:?} in English, got: {html}"
            );
        }
    }
}
