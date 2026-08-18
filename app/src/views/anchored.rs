use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;
use kayzen_core::habit_management::domain::habit::Habit;
use kayzen_core::habit_management::queries::list_anchored_habits::AnchoredScreen;
use kayzen_core::habit_management::use_cases::readmit_habit::ReadmitHabitError;

#[component]
pub fn Anchored() -> Element {
    let services = use_context::<Services>();
    let screen = services.list_anchored_habits.handle();
    let count = screen.habits.len();
    let max = Habit::MAX_IN_DAILY_LIFE;
    let mut readmit_error: Signal<Option<(String, &'static str)>> = use_signal(|| None);

    rsx! {
        div { class: "screen",
            header { class: "masthead",
                Link { class: "quiet-link", to: Route::Today {}, "← Aujourd'hui" }
            }
            h1 { class: "greeting", "Ancrées" }
            ul { class: "habit-list",
                for habit in screen.habits {
                    li { class: "habit-row",
                        span { class: "habit-name", "{habit.title}" }
                        button {
                            class: "readmit",
                            onclick: {
                                let services = services.clone();
                                let habit_id = habit.id.clone();
                                move |_| {
                                    let (result, _) = readmit_and_relist(&services, &habit_id);
                                    match result {
                                        Ok(()) => readmit_error.set(None),
                                        Err(error) => match refusal_message(error) {
                                            Some(message) => {
                                                readmit_error.set(Some((habit_id.clone(), message)))
                                            }
                                            None => readmit_error.set(None),
                                        },
                                    }
                                }
                            },
                            "La remettre dans mon quotidien"
                        }
                        if let Some((_, message)) = readmit_error()
                            .as_ref()
                            .filter(|(row_id, _)| row_id == &habit.id)
                        {
                            p { class: "quiet-note", "{message}" }
                        }
                    }
                }
            }
            p { class: "tally", "{count} · devenues naturelles" }
            p { class: "tally",
                "Vous suivez {screen.in_daily_life} / {max} habitudes en parallèle"
            }
        }
    }
}

// The tuple pairs a `#[must_use]` Result with the plain screen read-model;
// the attribute must stay for the screen half, so the redundancy lint is
// silenced rather than the annotation dropped (adr-0009 gesture pattern).
#[must_use]
#[allow(clippy::double_must_use)]
fn readmit_and_relist(
    services: &Services,
    id: &str,
) -> (Result<(), ReadmitHabitError>, AnchoredScreen) {
    let result = services.readmit_habit.execute(id);
    let screen = services.list_anchored_habits.handle();
    (result, screen)
}

#[must_use]
fn refusal_message(error: ReadmitHabitError) -> Option<&'static str> {
    match error {
        ReadmitHabitError::DailyLifeFull { .. } => {
            Some("Le quotidien est complet · pour la remettre, ancréez-en une autre d'abord")
        }
        ReadmitHabitError::DuplicateHabit => Some("Elle est déjà dans votre quotidien"),
        ReadmitHabitError::HabitNotFound | ReadmitHabitError::NotAnchored => None,
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
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/anchored")));
        });
        use_context_provider(services_with_two_anchored_habits);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtAnchoredScreenWithDailyLife() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/anchored")));
        });
        use_context_provider(services_with_three_non_anchored_and_one_anchored_habit);
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

    #[test]
    fn the_ancrees_screen_offers_to_readmit_each_anchored_habit() {
        let html = render(RootAtAnchoredScreen);

        assert_eq!(
            html.matches("La remettre dans mon quotidien").count(),
            2,
            "expected one readmit button per anchored habit, got: {html}"
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
    fn readmit_and_relist_removes_the_habit_from_the_screen_and_grows_the_parallel_count() {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", "Bouger un peu"));
        repository.save(&a_habit("h-2", "Boire un verre d'eau"));
        let mut anchored = a_habit("h-3", "Lire une page");
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        let services = Services::with_repository(repository);

        let (result, screen) = readmit_and_relist(&services, "h-3");

        assert_eq!(result, Ok(()));
        assert!(
            screen.habits.is_empty(),
            "the readmitted habit leaves the screen"
        );
        assert_eq!(screen.in_daily_life, 3);
    }

    #[test]
    fn readmit_and_relist_keeps_the_habit_listed_on_a_refusal() {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        for n in 1..=Habit::MAX_IN_DAILY_LIFE {
            repository.save(&a_habit(&format!("h-{n}"), &format!("Habit number {n}")));
        }
        let mut anchored = a_habit("h-anchored", "Lire une page");
        anchored.anchor().expect("a fresh habit is active");
        repository.save(&anchored);
        let services = Services::with_repository(repository);

        let (result, screen) = readmit_and_relist(&services, "h-anchored");

        assert_eq!(
            result,
            Err(ReadmitHabitError::DailyLifeFull {
                max: Habit::MAX_IN_DAILY_LIFE
            })
        );
        assert_eq!(screen.habits.len(), 1, "the refused habit stays listed");
        assert_eq!(screen.habits[0].id, "h-anchored");
    }

    #[test]
    fn a_full_daily_life_refusal_names_the_exact_quiet_message() {
        assert_eq!(
            refusal_message(ReadmitHabitError::DailyLifeFull { max: 5 }),
            Some("Le quotidien est complet · pour la remettre, ancréez-en une autre d'abord")
        );
    }

    #[test]
    fn a_duplicate_title_refusal_names_the_exact_quiet_message() {
        assert_eq!(
            refusal_message(ReadmitHabitError::DuplicateHabit),
            Some("Elle est déjà dans votre quotidien")
        );
    }

    #[test]
    fn an_unreachable_refusal_renders_no_message() {
        assert_eq!(refusal_message(ReadmitHabitError::HabitNotFound), None);
        assert_eq!(refusal_message(ReadmitHabitError::NotAnchored), None);
    }
}
