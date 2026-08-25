use crate::composition::Services;
use crate::i18n::{tr, tr_key};
use crate::route::Route;
use dioxus::prelude::*;
use kayzen_core::habit_management::domain::habit::Habit;
use kayzen_core::habit_management::queries::list_anchored_habits::AnchoredScreen;
use kayzen_core::habit_management::use_cases::readmit_habit::ReadmitHabitError;

#[component]
pub fn Anchored() -> Element {
    let services = use_context::<Services>();
    let mut screen = use_signal({
        let services = services.clone();
        move || services.list_anchored_habits.handle()
    });
    let count = screen().habits.len() as i64;
    let max = Habit::MAX_IN_DAILY_LIFE as i64;
    let mut readmit_error: Signal<Option<(String, &'static str)>> = use_signal(|| None);

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                Link { class: "quiet-link", to: Route::Today {}, {tr!("masthead-back-to-today")} }
            }
            h1 { class: "greeting", {tr!("anchored-heading")} }
            ul { class: "habit-list",
                for habit in screen().habits {
                    li { key: "{habit.id}", class: "habit-row",
                        div { class: "habit-body",
                            span { class: "habit-name", "{habit.title}" }
                            if let Some((_, message_key)) = readmit_error()
                                .as_ref()
                                .filter(|(row_id, _)| row_id == &habit.id)
                            {
                                p { class: "quiet-note", {tr_key(message_key)} }
                            }
                        }
                        button {
                            class: "readmit",
                            aria_label: tr!("anchored-readmit-aria", title: habit.title.clone()),
                            onclick: {
                                let services = services.clone();
                                let habit_id = habit.id.clone();
                                move |_| {
                                    let (reloaded, message) =
                                        readmit_and_relist(&services, &habit_id);
                                    screen.set(reloaded);
                                    readmit_error.set(message.map(|key| (habit_id.clone(), key)));
                                }
                            },
                            {tr!("anchored-readmit-label")}
                        }
                    }
                }
            }
            p { class: "tally", {tr!("anchored-count-tally", count: count)} }
            p { class: "tally", {tr!("anchored-daily-life-tally", count: screen().in_daily_life as i64, max: max)} }
        }
    }
}

#[must_use]
fn refusal_message_key(error: ReadmitHabitError) -> Option<&'static str> {
    match error {
        ReadmitHabitError::DailyLifeFull { .. } => Some("anchored-refusal-full"),
        ReadmitHabitError::DuplicateHabit => Some("anchored-refusal-duplicate"),
        ReadmitHabitError::HabitNotFound | ReadmitHabitError::NotAnchored => None,
    }
}

#[must_use]
fn readmit_and_relist(services: &Services, id: &str) -> (AnchoredScreen, Option<&'static str>) {
    let message = match services.readmit_habit.execute(id) {
        Ok(()) => None,
        Err(error) => refusal_message_key(error),
    };
    let screen = services.list_anchored_habits.handle();
    (screen, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::Services;
    use crate::i18n::{use_locale_for_tests, use_locale_for_tests_as};
    use crate::views::click_harness::Screen;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use dioxus_i18n::unic_langid::langid;
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
        first.anchor().expect("a fresh habit is active");
        repository.save(&first);
        let mut second = a_habit("h-2", "Bouger un peu");
        second.anchor().expect("a fresh habit is active");
        repository.save(&second);
        Services::with_repository(repository)
    }

    /// Two active, one paused, one anchored — the readmit-habit/S4 fixture:
    /// the daily life holds 3 non-anchored habits (a paused one counts), while
    /// the Ancrées screen lists exactly the one anchored habit.
    fn services_with_three_non_anchored_and_one_anchored_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", "Bouger un peu"));
        repository.save(&a_habit("h-2", "Boire un verre d'eau"));
        let mut paused = a_habit("h-3", "Respirer");
        paused.pause().expect("a fresh habit is active");
        repository.save(&paused);
        let mut anchored = a_habit("h-4", "Lire une page");
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        Services::with_repository(repository)
    }

    #[component]
    fn RootAtAnchoredScreen() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/anchored")));
        });
        use_context_provider(services_with_two_anchored_habits);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtAnchoredScreenAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/anchored")));
        });
        use_context_provider(services_with_two_anchored_habits);
        rsx! {
            Router::<Route> {}
        }
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_anchored_screen_in_english() {
        let html = render(RootAtAnchoredScreenAndEnglishLocale);

        assert!(
            html.contains(r#"<h1 class="greeting">Anchored</h1>"#),
            "expected the Anchored heading in English, got: {html}"
        );
        assert!(
            html.contains(">Bring it back into my daily life<"),
            "expected the readmit gesture label in English, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Bring it back into my daily life · Lire une page""#),
            "expected the readmit aria-label in English, got: {html}"
        );
        assert!(
            html.contains(r#"class="tally">2 · became natural<"#),
            "expected the anchored-count tally in English, got: {html}"
        );
        assert!(
            html.contains("You&#39;re following 0 / 5 habits in parallel"),
            "expected the daily-life tally in English, got: {html}"
        );
    }

    #[component]
    fn RootAtAnchoredScreenWithDailyLife() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/anchored")));
        });
        use_context_provider(services_with_three_non_anchored_and_one_anchored_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    fn services_with_full_daily_life_and_two_anchored_habits() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        for n in 1..=Habit::MAX_IN_DAILY_LIFE {
            repository.save(&a_habit(&format!("h-{n}"), &format!("Habit number {n}")));
        }
        let mut anchored = a_habit("h-anchored", "Lire une page");
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        let mut other_anchored = a_habit("h-anchored-2", "Bouger un peu");
        other_anchored.anchor().expect("a fresh habit is active");
        repository.save(&other_anchored);
        Services::with_repository(repository)
    }

    fn services_with_a_duplicate_titled_habit_and_one_anchored() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", "lire une page"));
        let mut anchored = a_habit("h-anchored", "Lire une page");
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        Services::with_repository(repository)
    }

    #[component]
    fn RootAtAnchoredScreenWithFullDailyLife() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/anchored")));
        });
        use_context_provider(services_with_full_daily_life_and_two_anchored_habits);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtAnchoredScreenWithDuplicateTitle() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/anchored")));
        });
        use_context_provider(services_with_a_duplicate_titled_habit_and_one_anchored);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtEmptyAnchoredScreen() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/anchored")));
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

    #[test]
    fn clicking_readmit_takes_the_habit_out_of_the_anchored_list_and_grows_the_parallel_count() {
        let mut screen = Screen::open(RootAtAnchoredScreen);

        screen.click("La remettre dans mon quotidien · Lire une page");

        let html = screen.html();
        assert!(
            !html.contains("Lire une page"),
            "the readmitted habit leaves the Ancrées list, got: {html}"
        );
        assert!(
            html.contains("Bouger un peu"),
            "the other anchored habit stays, got: {html}"
        );
        assert!(html.contains("1 · devenues naturelles"), "got: {html}");
        assert!(
            html.contains("Vous suivez 1 / 5 habitudes en parallèle"),
            "got: {html}"
        );
    }

    #[test]
    fn clicking_readmit_on_a_full_daily_life_shows_the_refusal_only_under_the_refused_row() {
        let mut screen = Screen::open(RootAtAnchoredScreenWithFullDailyLife);

        screen.click("La remettre dans mon quotidien · Lire une page");

        let html = screen.html();
        let message =
            "Le quotidien est complet · pour la remettre, ancrez-en une autre d&#39;abord";
        assert_eq!(
            html.matches(message).count(),
            1,
            "expected the refusal message under exactly the refused row, got: {html}"
        );
        let message_index = html.find(message).expect("refusal message present");
        let other_row_index = html
            .find("Bouger un peu")
            .expect("the other anchored habit still renders");
        assert!(
            message_index < other_row_index,
            "expected the refusal message under Lire une page's row, not Bouger un peu's, got: {html}"
        );
    }

    #[test]
    fn clicking_readmit_on_a_duplicate_title_shows_the_refusal_message() {
        let mut screen = Screen::open(RootAtAnchoredScreenWithDuplicateTitle);

        screen.click("La remettre dans mon quotidien · Lire une page");

        let html = screen.html();
        assert!(
            html.contains("Elle est déjà dans votre quotidien"),
            "expected the duplicate refusal message next to the refused row, got: {html}"
        );
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

    #[test]
    fn the_ancrees_screen_offers_to_readmit_each_anchored_habit() {
        let html = render(RootAtAnchoredScreen);

        assert!(
            html.contains(r#"aria-label="La remettre dans mon quotidien · Lire une page""#),
            "expected a readmit button named after Lire une page, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="La remettre dans mon quotidien · Bouger un peu""#),
            "expected a readmit button named after Bouger un peu, got: {html}"
        );
    }

    // @scenario: readmit-habit/S4
    #[test]
    fn the_ancrees_screen_states_how_many_habits_are_followed_in_parallel() {
        let html = render(RootAtAnchoredScreenWithDailyLife);

        assert!(
            html.contains(&format!(
                "Vous suivez 3 / {} habitudes en parallèle",
                Habit::MAX_IN_DAILY_LIFE
            )),
            "expected the parallel-count line to read 3 non-anchored habits (a paused one counts), \
             got: {html}"
        );
    }

    #[test]
    fn the_parallel_count_footer_still_renders_when_nothing_is_anchored() {
        let html = render(RootAtEmptyAnchoredScreen);

        assert!(
            html.contains("Vous suivez 0 / 5 habitudes en parallèle"),
            "expected the parallel-count footer even on an empty Ancrées list, got: {html}"
        );
    }

    #[test]
    fn readmit_and_relist_removes_the_habit_from_the_screen_grows_the_parallel_count_and_clears_the_message()
     {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", "Bouger un peu"));
        repository.save(&a_habit("h-2", "Boire un verre d'eau"));
        let mut anchored = a_habit("h-3", "Lire une page");
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        let services = Services::with_repository(repository);

        let (screen, message) = readmit_and_relist(&services, "h-3");

        assert_eq!(message, None, "a successful readmit carries no refusal");
        assert!(
            screen.habits.is_empty(),
            "the readmitted habit leaves the screen"
        );
        assert_eq!(screen.in_daily_life, 3);
    }

    #[test]
    fn readmit_and_relist_keeps_the_habit_listed_and_names_the_full_daily_life_message_on_refusal()
    {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        for n in 1..=Habit::MAX_IN_DAILY_LIFE {
            repository.save(&a_habit(&format!("h-{n}"), &format!("Habit number {n}")));
        }
        let mut anchored = a_habit("h-anchored", "Lire une page");
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        let services = Services::with_repository(repository);

        let (screen, message_key) = readmit_and_relist(&services, "h-anchored");

        assert_eq!(message_key, Some("anchored-refusal-full"));
        assert_eq!(screen.habits.len(), 1, "the refused habit stays listed");
        assert_eq!(screen.habits[0].id, "h-anchored");
    }

    #[test]
    fn readmit_and_relist_keeps_the_habit_listed_and_names_the_duplicate_message_on_refusal() {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", "lire une page"));
        let mut anchored = a_habit("h-anchored", "Lire une page");
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        let services = Services::with_repository(repository);

        let (screen, message_key) = readmit_and_relist(&services, "h-anchored");

        assert_eq!(message_key, Some("anchored-refusal-duplicate"));
        assert_eq!(screen.habits.len(), 1, "the refused habit stays listed");
        assert_eq!(screen.habits[0].id, "h-anchored");
    }

    #[test]
    fn a_full_daily_life_refusal_names_the_exact_catalogue_key() {
        assert_eq!(
            refusal_message_key(ReadmitHabitError::DailyLifeFull { max: 5 }),
            Some("anchored-refusal-full")
        );
    }

    #[test]
    fn a_duplicate_title_refusal_names_the_exact_catalogue_key() {
        assert_eq!(
            refusal_message_key(ReadmitHabitError::DuplicateHabit),
            Some("anchored-refusal-duplicate")
        );
    }

    #[test]
    fn an_unreachable_refusal_renders_no_message() {
        assert_eq!(refusal_message_key(ReadmitHabitError::HabitNotFound), None);
        assert_eq!(refusal_message_key(ReadmitHabitError::NotAnchored), None);
    }

    #[test]
    fn every_refusal_key_resolves_in_both_catalogues() {
        let (fr_ids, en_ids) = crate::i18n::catalogue_ids();

        for error in ReadmitHabitError::ALL {
            let Some(key) = refusal_message_key(error) else {
                continue;
            };
            assert!(fr_ids.contains(key), "expected {key} to resolve in fr.ftl");
            assert!(en_ids.contains(key), "expected {key} to resolve in en.ftl");
        }
    }

    #[test]
    fn the_ancrees_row_wraps_the_habit_name_in_the_habit_body_layout_wrapper() {
        let html = render(RootAtAnchoredScreen);

        assert!(
            html.contains(
                r#"<div class="habit-body"><span class="habit-name">Lire une page</span></div>"#
            ),
            "expected the habit name wrapped in the house habit-body layout element \
             (see today.rs), got: {html}"
        );
    }

    #[test]
    fn the_refusal_note_renders_inside_the_habit_body_wrapper_not_as_a_flex_row_sibling() {
        let mut screen = Screen::open(RootAtAnchoredScreenWithFullDailyLife);

        screen.click("La remettre dans mon quotidien · Lire une page");

        let html = screen.html();
        let message =
            "Le quotidien est complet · pour la remettre, ancrez-en une autre d&#39;abord";
        assert!(
            html.contains(&format!(
                r#"<p class="quiet-note">{message}</p></div><button"#
            )),
            "expected the refusal note nested inside the habit-body wrapper, closed before \
             the readmit button — not a direct child of the flex habit-row, got: {html}"
        );
    }
}
