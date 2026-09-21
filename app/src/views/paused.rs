use crate::composition::Services;
use crate::i18n::tr;
use crate::route::Route;
use dioxus::prelude::*;
use kayzen_core::habit_management::queries::list_board_habits::PausedHabit;

#[component]
pub fn Paused() -> Element {
    let services = use_context::<Services>();
    let mut habits = use_signal({
        let services = services.clone();
        move || services.list_paused_habits.handle()
    });
    let paused = habits();

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                Link { class: "quiet-link", to: Route::Today {}, {tr!("masthead-back-to-today")} }
            }
            h1 { class: "greeting", {tr!("paused-heading")} }
            if paused.is_empty() {
                p { class: "quiet-note", {tr!("paused-empty-note")} }
            } else {
                ul { class: "habit-list",
                    for habit in paused {
                        li { key: "{habit.id}", class: "habit-row",
                            div { class: "habit-body",
                                Link {
                                    class: "habit-name",
                                    to: Route::HabitDetail { id: habit.id.clone() },
                                    "{habit.title}"
                                }
                            }
                            button {
                                class: "resume-link",
                                aria_label: tr!("resume-habit-aria", title: habit.title.clone()),
                                onclick: {
                                    let services = services.clone();
                                    let id = habit.id.clone();
                                    move |_| habits.set(resume_and_relist(&services, &id))
                                },
                                {tr!("resume-habit-label")}
                            }
                        }
                    }
                }
            }
        }
    }
}

#[must_use]
fn resume_and_relist(services: &Services, id: &str) -> Vec<PausedHabit> {
    services.resume_habit.execute(id).ok();
    services.list_paused_habits.handle()
}

#[cfg(test)]
mod tests {
    use super::*;
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
    use kayzen_core::shared::clock::Clock;
    use kayzen_core::shared::local_date::LocalDate;
    use std::rc::Rc;

    struct FixedClock(LocalDate);

    impl Clock for FixedClock {
        fn today(&self) -> LocalDate {
            self.0
        }
    }

    fn a_habit(id: &str, title: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new(title.to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        )
    }

    fn paused_habit(id: &str, title: &str) -> Habit {
        let mut habit = a_habit(id, title);
        habit.pause().expect("a fresh habit is active");
        habit
    }

    fn services_with_two_paused_habits() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&paused_habit("h-1", "Lire une page"));
        repository.save(&paused_habit("h-2", "Bouger un peu"));
        Services::with_repository(repository)
    }

    #[component]
    fn RootAtPausedScreen() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/paused")));
        });
        use_context_provider(services_with_two_paused_habits);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtEmptyPausedScreen() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/paused")));
        });
        use_context_provider(|| Services::with_repository(Rc::new(InMemoryHabitRepository::new())));
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtPausedScreenAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/paused")));
        });
        use_context_provider(services_with_two_paused_habits);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtEmptyPausedScreenAndEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/paused")));
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

    // @scenario: paused-habits/S1
    #[test]
    fn the_paused_screen_lists_each_habit_at_rest_with_its_resume_affordance() {
        let html = render(RootAtPausedScreen);

        assert!(
            html.contains(r#"<h1 class="greeting">En pause</h1>"#),
            "expected the paused heading, got: {html}"
        );
        assert!(
            html.contains("Lire une page") && html.contains("Bouger un peu"),
            "expected both habits at rest to be listed, got: {html}"
        );
        assert!(
            html.contains(r#"href="/habit/h-1""#) && html.contains(r#"href="/habit/h-2""#),
            "expected each habit at rest to link to its own detail screen, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="La reprendre · Lire une page""#),
            "expected a resume affordance named after Lire une page, got: {html}"
        );
        assert!(
            html.contains(r#"aria-label="La reprendre · Bouger un peu""#),
            "expected a resume affordance named after Bouger un peu, got: {html}"
        );
        assert!(
            html.contains(">La reprendre<"),
            "expected the resume label on each row, got: {html}"
        );
    }

    // @scenario: paused-habits/S1
    #[test]
    fn a_paused_screen_with_nothing_at_rest_says_so_instead_of_listing_nothing() {
        let html = render(RootAtEmptyPausedScreen);

        assert!(
            html.contains("Rien ici. Tout est de retour dans votre quotidien."),
            "expected the empty-note copy, got: {html}"
        );
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_paused_screen_in_english() {
        let html = render(RootAtPausedScreenAndEnglishLocale);

        assert!(
            html.contains(r#"<h1 class="greeting">Paused</h1>"#),
            "expected the paused heading in English, got: {html}"
        );
        assert!(
            html.contains(">Resume it<")
                && html.contains(r#"aria-label="Resume it · Lire une page""#),
            "expected the resume affordance in English, got: {html}"
        );
        assert!(
            !html.contains("La reprendre") && !html.contains("En pause"),
            "expected no leftover French copy under an English locale, got: {html}"
        );
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_empty_paused_screen_in_english() {
        let html = render(RootAtEmptyPausedScreenAndEnglishLocale);

        assert!(
            html.contains("Nothing here. Everything is back in your daily life."),
            "expected the empty-note copy in English, got: {html}"
        );
    }

    // @scenario: paused-habits/S2
    #[test]
    fn clicking_la_reprendre_removes_the_habit_from_the_paused_screen() {
        let mut screen = Screen::open(RootAtPausedScreen);

        screen.click("La reprendre · Lire une page");

        let html = screen.html();
        assert!(
            !html.contains("Lire une page"),
            "expected the resumed habit to leave the paused screen, got: {html}"
        );
        assert!(
            html.contains("Bouger un peu"),
            "expected the other habit at rest to stay listed, got: {html}"
        );
    }

    // @scenario: pause-resume/S2
    #[test]
    fn resuming_from_the_paused_screen_leaves_the_completion_history_untouched() {
        let clock: Rc<dyn Clock> = Rc::new(FixedClock(LocalDate::from_epoch_day(20_005)));
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", "Lire une page");
        habit.toggle_done(clock.today());
        habit.pause().expect("a fresh habit is active");
        repository.save(&habit);
        let services = Services::with_repository_and_clock(repository, clock);

        let paused = super::resume_and_relist(&services, "h-1");

        assert!(
            paused.is_empty(),
            "expected the resumed habit to leave the paused list, got: {paused:?}"
        );
        let detail = services
            .get_habit_detail
            .handle("h-1")
            .expect("the habit survives the resume");
        assert!(
            detail.done_today,
            "expected the completion recorded before the pause to survive the resume"
        );
    }
}
