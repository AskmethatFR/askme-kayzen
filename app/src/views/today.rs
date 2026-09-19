use crate::composition::Services;
use crate::i18n::tr;
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
    let parts = services.today_calendar_parts();

    let today_habits = habits();
    let total = today_habits.active.len();
    let done = today_habits
        .active
        .iter()
        .filter(|habit| habit.done_today)
        .count();
    let paused_count = today_habits.paused.len();
    let has_active_habits = !today_habits.active.is_empty();

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                span {
                    class: "masthead-date",
                    {tr!("today-date", weekday: parts.weekday as i64, day: parts.day as i64, month: parts.month as i64)}
                }
            }
            h1 { class: "greeting", {tr!("today-greeting")} }

            if today_habits.is_empty() {
                div { class: "empty-state",
                    p { class: "lede", {tr!("today-empty-lede-1")} }
                    p { class: "lede", {tr!("today-empty-lede-2")} }
                    div { class: "add-cta",
                        Link {
                            class: "quiet-link",
                            to: Route::AddHabit {},
                            {tr!("today-add-cta")}
                        }
                    }
                }
            } else {
                p { class: "lede", {tr!("today-lede")} }

                if has_active_habits {
                    div { class: "summary-card",
                        for habit in today_habits.active.iter() {
                            span {
                                key: "{habit.id}",
                                class: if habit.done_today { "pebble is-done" } else { "pebble" },
                            }
                        }
                        p {
                            class: "summary-tally",
                            {tr!("today-tally", done: done as i64, total: total as i64)}
                        }
                    }

                    p { class: "eyebrow", {tr!("today-eyebrow-active")} }
                    ul { class: "habit-list",
                        for habit in today_habits.active {
                            li { key: "{habit.id}", class: "habit-row",
                                div { class: "habit-body",
                                    Link {
                                        class: "habit-name",
                                        to: Route::HabitDetail { id: habit.id.clone() },
                                        "{habit.title}"
                                    }
                                    div {
                                        class: "habit-meta",
                                        {tr!("today-habit-meta", minutes: habit.minutes as i64)}
                                    }
                                }
                                button {
                                    class: if habit.done_today { "target is-done" } else { "target" },
                                    aria_label: if habit.done_today { tr!("today-done-aria", title: habit.title.clone()) } else { tr!("today-mark-done-aria", title: habit.title.clone()) },
                                    onclick: {
                                        let services = services.clone();
                                        let id = habit.id.clone();
                                        move |_| habits.set(mark_done_and_relist(&services, &id))
                                    },
                                    span { class: "target-ink" }
                                }
                            }
                        }
                    }
                }

                if has_active_habits || !today_habits.paused.is_empty() {
                    div { class: "footer-links",
                        if has_active_habits {
                            Link {
                                class: "quiet-link",
                                to: Route::Week {},
                                {tr!("today-week-link")}
                                span { class: "week-link-arrow", "aria-hidden": "true", "→" }
                            }
                        }
                        if !today_habits.paused.is_empty() {
                            Link {
                                class: "quiet-link",
                                to: Route::Paused {},
                                {tr!("today-paused-link", count: paused_count as i64)}
                            }
                        }
                    }
                }
                div { class: "add-cta",
                    Link {
                        class: "quiet-link",
                        to: Route::AddHabit {},
                        {tr!("today-add-cta")}
                    }
                }
            }
        }
    }
}

#[must_use]
fn mark_done_and_relist(services: &Services, id: &str) -> TodayHabits {
    services.mark_done.execute(id).ok();
    services.list_board_habits.handle()
}

#[cfg(test)]
mod tests {
    use crate::composition::Services;
    use crate::i18n::{select_locale, use_locale_for_tests, use_locale_for_tests_as};
    use crate::route::Route;
    use crate::views::click_harness::Screen;
    use dioxus::prelude::*;
    use dioxus_i18n::unic_langid::langid;
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

    fn a_second_habit() -> Habit {
        Habit::new(
            HabitId::new("test-2").unwrap(),
            HabitTitle::new("Move a little".to_string()).unwrap(),
            Goal::new(2).unwrap(),
            LocalDate::from_epoch_day(20_000),
        )
    }

    fn a_paused_habit() -> Habit {
        let mut paused = Habit::new(
            HabitId::new("test-3").unwrap(),
            HabitTitle::new("Breathe".to_string()).unwrap(),
            Goal::new(2).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        paused.pause().expect("a fresh habit is active");
        paused
    }

    fn services_with_one_undone_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        Services::with_repository(repository)
    }

    fn services_with_no_habit() -> Services {
        Services::with_repository(Rc::new(InMemoryHabitRepository::new()))
    }

    /// Saturday 19 September 2026 — the mockup's own date, and the one the
    /// delta spec's convention test pins (`calendar_parts(739_878)`).
    fn services_with_one_undone_habit_on_the_mockup_date() -> Services {
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(739_878)));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        Services::with_repository_and_clock(repository, clock)
    }

    fn services_with_one_habit_done_today() -> Services {
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_005)));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit();
        habit.toggle_done(clock.today());
        repository.save(&habit);
        Services::with_repository_and_clock(repository, clock)
    }

    fn services_with_one_anchored_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut anchored = a_habit();
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        Services::with_repository(repository)
    }

    fn services_with_three_active_two_done_and_one_paused_habit() -> Services {
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_005)));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut first_done = a_habit();
        first_done.toggle_done(clock.today());
        repository.save(&first_done);
        repository.save(&a_second_habit());
        let mut last_done = Habit::new(
            HabitId::new("test-4").unwrap(),
            HabitTitle::new("Write a line".to_string()).unwrap(),
            Goal::new(3).unwrap(),
            LocalDate::from_epoch_day(20_000),
        );
        last_done.toggle_done(clock.today());
        repository.save(&last_done);
        repository.save(&a_paused_habit());
        Services::with_repository_and_clock(repository, clock)
    }

    fn services_with_one_paused_habit_only() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_paused_habit());
        Services::with_repository(repository)
    }

    #[component]
    fn RootWithUndoneHabit() -> Element {
        use_locale_for_tests();
        use_context_provider(services_with_one_undone_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithNoHabit() -> Element {
        use_locale_for_tests();
        use_context_provider(services_with_no_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithHabitDoneToday() -> Element {
        use_locale_for_tests();
        use_context_provider(services_with_one_habit_done_today);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithThreeActiveAndOnePausedHabit() -> Element {
        use_locale_for_tests();
        use_context_provider(services_with_three_active_two_done_and_one_paused_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithOnePausedHabitOnly() -> Element {
        use_locale_for_tests();
        use_context_provider(services_with_one_paused_habit_only);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithAnchoredHabit() -> Element {
        use_locale_for_tests();
        use_context_provider(services_with_one_anchored_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithUndoneHabitAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_context_provider(services_with_one_undone_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithNoHabitAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_context_provider(services_with_no_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithThreeActiveAndOnePausedHabitAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_context_provider(services_with_three_active_two_done_and_one_paused_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithUndoneHabitAndUnsupportedLocale() -> Element {
        use_locale_for_tests_as(select_locale(Some("de-DE")));
        use_context_provider(services_with_one_undone_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithUndoneHabitAndNoLocaleReported() -> Element {
        use_locale_for_tests_as(select_locale(None));
        use_context_provider(services_with_one_undone_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithUndoneHabitOnTheMockupDate() -> Element {
        use_locale_for_tests();
        use_context_provider(services_with_one_undone_habit_on_the_mockup_date);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootWithUndoneHabitOnTheMockupDateAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_context_provider(services_with_one_undone_habit_on_the_mockup_date);
        rsx! {
            Router::<Route> {}
        }
    }

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    fn footer_links_section(html: &str) -> &str {
        let start = html
            .find(r#"<div class="footer-links">"#)
            .expect("expected a footer-links wrapper around the footer navigation links");
        let after_open = &html[start..];
        let end = after_open
            .find("</div>")
            .expect("expected the footer-links wrapper to close");
        &after_open[..end]
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_active_board_in_english() {
        let html = render(RootWithUndoneHabitAndEnglishLocale);

        assert!(
            html.contains("Hello."),
            "expected the greeting in English, got: {html}"
        );
        assert!(
            html.contains("Your small steps"),
            "expected the active-habits eyebrow in English, got: {html}"
        );
        assert!(
            html.contains("every day") && html.contains("4 min"),
            "expected the habit meta line in English, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Mark as done · Read one page""#),
            "expected the mark-done aria-label in English, got: {html}"
        );
        assert!(
            html.contains("0 of 1 ·") && html.contains("that&#39;s already something."),
            "expected the tally in English, got: {html}"
        );
        assert!(
            html.contains("See how I&#39;m growing · this week"),
            "expected the week link in English, got: {html}"
        );
        assert!(
            !html.contains("Bonjour") && !html.contains("Aujourd"),
            "expected no leftover French copy under an English locale, got: {html}"
        );
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_dates_the_masthead_in_english() {
        let html = render(RootWithUndoneHabitOnTheMockupDateAndEnglishLocale);

        assert!(
            html.contains("Saturday 19 September"),
            "expected the masthead to name the day in English, got: {html}"
        );
    }

    // @scenario: today-habit-list/S8
    #[test]
    fn the_masthead_names_the_day_it_is_in_french() {
        let html = render(RootWithUndoneHabitOnTheMockupDate);
        let masthead = &html[..html
            .find(r#"<h1 class="greeting""#)
            .expect("expected the masthead to close before the greeting")];

        assert!(
            masthead.contains(r#"<span class="masthead-date">Samedi 19 septembre</span>"#),
            "expected the masthead to read the date, got: {html}"
        );
        assert!(
            !masthead.contains("Aujourd"),
            "expected the masthead to stop reading « Aujourd'hui », got: {masthead}"
        );
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_empty_board_invitation_in_english() {
        let html = render(RootWithNoHabitAndEnglishLocale);

        assert!(
            html.contains("Nothing yet. And that&#39;s perfectly fine.")
                && html.contains("One tiny habit is enough to start."),
            "expected the empty-state invitation in English, got: {html}"
        );
        assert!(
            html.contains("+ Add a tiny habit"),
            "expected the add-habit gesture in English, got: {html}"
        );
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_today_and_its_paused_link_in_english() {
        let html = render(RootWithThreeActiveAndOnePausedHabitAndEnglishLocale);

        assert!(
            html.contains("Today") && html.contains("Hello."),
            "expected the masthead date and greeting in English, got: {html}"
        );
        assert!(
            html.contains("Your small steps") && html.contains("2 of 3 ·"),
            "expected the active board in English, got: {html}"
        );
        assert!(
            html.contains("1 paused · no pressure"),
            "expected the paused link's copy in English, got: {html}"
        );
        assert!(
            !html.contains("Mes habitudes ancr"),
            "expected no Ancrées link on Today any more, got: {html}"
        );
        assert!(
            !html.contains("Bonjour") && !html.contains("Aujourd"),
            "expected no leftover French copy under an English locale, got: {html}"
        );
    }

    // @scenario: language/S2
    #[test]
    fn a_locale_the_app_carries_no_catalogue_for_falls_back_to_french() {
        let html = render(RootWithUndoneHabitAndUnsupportedLocale);

        assert!(
            html.contains("Bonjour."),
            "expected the French greeting when the device locale isn't carried, got: {html}"
        );
    }

    // @scenario: language/S3
    #[test]
    fn no_locale_reported_at_all_falls_back_to_french() {
        let html = render(RootWithUndoneHabitAndNoLocaleReported);

        assert!(
            html.contains("Bonjour."),
            "expected the French greeting when the device reports no locale, got: {html}"
        );
    }

    #[test]
    fn clicking_mark_done_stamps_the_habit_and_grows_the_done_tally() {
        let mut screen = Screen::open(RootWithUndoneHabit);

        screen.click("Marquer comme fait · Read one page");

        let html = screen.html();
        assert!(
            html.contains("target is-done"),
            "expected the target to be stamped after the click, got: {html}"
        );
        assert!(
            html.contains(r#"class="pebble is-done""#),
            "expected the summary pebble to be filled after the click, got: {html}"
        );
        assert!(
            html.contains("1 sur 1 ·"),
            "expected the tally to count the freshly-done habit, got: {html}"
        );
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
        assert!(
            html.contains(r#"aria-label="Fait aujourd&#39;hui · Read one page""#),
            "expected the aria-label to name which habit is stamped, got: {html}"
        );
    }

    #[test]
    fn the_summary_card_fills_the_pebble_of_a_habit_done_today() {
        let html = render(RootWithHabitDoneToday);

        assert!(
            html.contains(r#"class="summary-card""#),
            "expected the summary card in the active board, got: {html}"
        );
        assert!(
            html.contains(r#"class="pebble is-done""#),
            "expected the done habit's pebble to be filled, got: {html}"
        );
        assert_eq!(
            html.matches(r#"class="pebble is-done""#).count(),
            1,
            "expected the done habit's pebble to be the only one, got: {html}"
        );
        assert_eq!(
            html.matches(r#"class="pebble""#).count(),
            0,
            "expected no empty pebble when the only active habit is done today, got: {html}"
        );
    }

    // @scenario: today-habit-list/S5
    #[test]
    fn a_paused_habit_leaves_today_for_its_own_screen_behind_a_link_naming_the_count() {
        let html = render(RootWithThreeActiveAndOnePausedHabit);

        assert!(
            html.contains("Read one page")
                && html.contains("Move a little")
                && html.contains("Write a line"),
            "expected the active habits to stay listed, got: {html}"
        );
        assert!(
            !html.contains("Breathe"),
            "expected the paused habit to leave Today entirely, got: {html}"
        );
        assert!(
            !html.contains("En pause"),
            "expected the paused zone to be gone from Today, got: {html}"
        );
        assert!(
            html.contains(r#"href="/paused""#) && html.contains("1 en pause · aucune pression"),
            "expected a link to the paused screen naming the paused count, got: {html}"
        );
        assert!(
            html.contains(
                r#"<span class="pebble is-done"></span><span class="pebble"></span><span class="pebble is-done"></span>"#
            ),
            "expected one pebble per active habit in board order, filled exactly for the two \
             done today, got: {html}"
        );
        assert_eq!(
            html.matches(r#"class="pebble is-done""#).count(),
            2,
            "expected two filled pebbles for the two active habits done today, got: {html}"
        );
        assert_eq!(
            html.matches(r#"class="pebble""#).count(),
            1,
            "expected one empty pebble for the one active habit not done today, got: {html}"
        );
        assert!(
            html.contains("2 sur 3 ·"),
            "expected the tally to count the active habits only, got: {html}"
        );
    }

    // @scenario: today-habit-list/S6
    #[test]
    fn no_paused_link_is_offered_when_nothing_is_paused() {
        let html = render(RootWithUndoneHabit);

        assert!(
            !html.contains(r#"href="/paused""#),
            "expected no link to the paused screen when nothing is in pause, got: {html}"
        );
        assert!(
            !html.contains("en pause"),
            "expected no paused link copy when nothing is in pause, got: {html}"
        );
    }

    // @scenario: today-habit-list/S7
    #[test]
    fn a_board_whose_only_habits_are_paused_keeps_the_invitation_to_add() {
        let html = render(RootWithOnePausedHabitOnly);

        assert!(
            !html.contains(r#"class="summary-card""#),
            "expected no summary card when nothing is active, got: {html}"
        );
        assert!(
            !html.contains("Vos petits pas"),
            "expected the habit-list heading to be hidden when nothing is active, got: {html}"
        );
        assert!(
            !html.contains("Voir comment je grandis"),
            "expected the week link to be hidden when nothing is active, got: {html}"
        );
        assert!(
            html.contains("Un seul petit pas suffit"),
            "expected the lede to stay visible, got: {html}"
        );
        assert!(
            html.contains(r#"href="/paused""#) && html.contains("1 en pause · aucune pression"),
            "expected the paused link to stay visible, got: {html}"
        );
        assert!(
            html.contains("+ Ajouter une toute petite habitude"),
            "expected the add-habit gesture to stay visible, got: {html}"
        );
    }

    #[test]
    fn the_footer_links_wrapper_holds_only_the_week_link_when_nothing_is_paused() {
        let html = render(RootWithUndoneHabit);
        let section = footer_links_section(&html);

        assert!(
            section.contains("Voir comment je grandis · cette semaine"),
            "expected the Week link inside the footer-links wrapper, got: {section}"
        );
        assert!(
            !section.contains("en pause"),
            "expected no paused link inside the wrapper when nothing is paused, got: {section}"
        );
    }

    #[test]
    fn the_footer_links_wrapper_stacks_the_week_and_paused_links_when_something_is_paused() {
        let html = render(RootWithThreeActiveAndOnePausedHabit);
        let section = footer_links_section(&html);

        assert!(
            section.contains("Voir comment je grandis · cette semaine"),
            "expected the Week link inside the footer-links wrapper, got: {section}"
        );
        assert!(
            section.contains("1 en pause · aucune pression"),
            "expected the paused link inside the same footer-links wrapper, got: {section}"
        );
    }

    #[test]
    fn today_renders_no_ancrees_link_even_when_a_habit_is_anchored() {
        let html = render(RootWithAnchoredHabit);

        assert!(
            !html.contains("Mes habitudes ancr"),
            "expected Today to carry no Ancrées link any more, got: {html}"
        );
    }

    #[test]
    fn the_week_link_carries_its_decorative_arrow() {
        let html = render(RootWithUndoneHabit);
        let section = footer_links_section(&html);

        assert!(
            section.contains(r#"class="week-link-arrow""#) && section.contains("→"),
            "expected the week link to carry its decorative arrow, got: {section}"
        );
    }

    // @scenario: persistence/S3
    // @scenario: today-habit-list/S4
    #[test]
    fn an_empty_board_shows_the_invitation_and_hides_the_summary_heading_and_week_link() {
        let html = render(RootWithNoHabit);

        assert!(
            html.contains("Rien pour l&#39;instant. Et c&#39;est très bien.")
                && html.contains("Une seule toute petite habitude suffit pour commencer."),
            "expected the empty-state invitation copy, got: {html}"
        );
        assert!(
            html.contains("+ Ajouter une toute petite habitude"),
            "expected the add-habit gesture to be offered, got: {html}"
        );
        let screen_content = &html[..html
            .find(r#"<nav class="bottom-nav""#)
            .expect("the bar renders on Today")];
        let interactive_elements =
            screen_content.matches("<a ").count() + screen_content.matches("<button").count();
        assert_eq!(
            interactive_elements, 1,
            "expected the add-habit gesture to be the only interactive element in the screen's content, got: {html}"
        );
        assert!(
            !html.contains(r#"class="summary-card""#),
            "expected no summary card on an empty board, got: {html}"
        );
        assert!(
            !html.contains("Vos petits pas"),
            "expected the habit-list eyebrow to be hidden, got: {html}"
        );
        assert!(
            !html.contains("Voir comment je grandis"),
            "expected the week link to be hidden on an empty board, got: {html}"
        );
    }

    // B4 / Dev-B F2: the previous assertion checked for a raw apostrophe
    // ("Rien pour l'instant"), but dioxus_ssr HTML-escapes it to `&#39;`
    // (proven by the sibling assertion at :418) — the needle was never
    // present whatever the view did, so this test could not fail. Asserting
    // on the `empty-state` class names the branch itself instead of a copy
    // string, so it survives future copy edits and cannot be fooled by
    // escaping.
    #[test]
    fn a_board_with_a_habit_does_not_render_the_empty_state() {
        let html = render(RootWithUndoneHabit);

        assert!(
            !html.contains(r#"class="empty-state""#),
            "expected the empty-state branch to stay absent once a habit exists, got: {html}"
        );
    }

    #[test]
    fn mark_done_and_relist_marks_the_habit_done_and_returns_the_refreshed_board() {
        let services = services_with_one_undone_habit();

        let board = super::mark_done_and_relist(&services, "test-1");

        assert!(
            board
                .active
                .iter()
                .any(|habit| habit.id == "test-1" && habit.done_today),
            "expected the habit to be marked done in the refreshed board, got: {board:?}"
        );
    }
}
