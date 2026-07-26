use crate::composition::Services;
use crate::route::Route;
use dioxus::prelude::*;
use kayzen_core::habit_management::queries::get_habit_detail::HabitDetail as HabitDetailData;

#[component]
pub fn HabitDetail(id: String) -> Element {
    let services = use_context::<Services>();
    let detail: Option<HabitDetailData> = services.get_habit_detail.handle(&id);

    match detail {
        Some(habit) => {
            let step_count = habit.steps.len();
            rsx! {
                div { class: "screen",
                    header { class: "masthead",
                        Link { class: "quiet-link", to: Route::Today {}, "← Aujourd'hui" }
                    }
                    h1 { class: "greeting", "{habit.title}" }
                    p { class: "lede", "chaque jour · {habit.current_goal} min" }

                    div {
                        class: "staircase",
                        "aria-label": "Escalier de progression, objectif actuel {habit.current_goal} minutes",
                        for (index , minutes) in habit.steps.iter().enumerate() {
                            span {
                                class: if index + 1 == step_count { "step-bar is-current" } else { "step-bar" },
                                style: "--step-minutes: {minutes}",
                            }
                        }
                    }

                    Link {
                        class: "btn btn-primary btn-block",
                        to: Route::Ritual { id: id.clone() },
                        "Faire ma minute"
                    }
                }
            }
        }
        None => rsx! {
            div { class: "screen",
                p { class: "lede", "Cette habitude n'est plus sur votre liste." }
                Link { class: "quiet-link", to: Route::Today {}, "Retour à Aujourd'hui" }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::history::{MemoryHistory, provide_history_context};
    use kayzen_core::habit_management::domain::goal::Goal;
    use kayzen_core::habit_management::domain::habit::Habit;
    use kayzen_core::habit_management::domain::habit_id::HabitId;
    use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
    use kayzen_core::habit_management::domain::habit_title::HabitTitle;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use kayzen_core::shared::local_date::LocalDate;
    use std::rc::Rc;

    fn a_habit() -> Habit {
        Habit::new(
            HabitId::new("h-1".to_string()),
            HabitTitle::new("Lire une page".to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(20_000),
        )
    }

    fn services_with_one_habit() -> Services {
        let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit());
        Services::with_repository(repository)
    }

    fn services_with_no_habits() -> Services {
        Services::with_repository(Rc::new(InMemoryHabitRepository::new()))
    }

    #[component]
    fn RootAtKnownHabit() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/h-1")));
        });
        use_context_provider(services_with_one_habit);
        rsx! {
            Router::<Route> {}
        }
    }

    #[component]
    fn RootAtUnknownHabit() -> Element {
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/habit/missing")));
        });
        use_context_provider(services_with_no_habits);
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
    fn a_known_habit_renders_its_title_goal_and_staircase() {
        let html = render(RootAtKnownHabit);

        assert!(
            html.contains("Lire une page"),
            "expected the habit title, got: {html}"
        );
        assert!(
            html.contains("chaque jour · 5 min"),
            "expected the dose, got: {html}"
        );
        assert!(
            html.contains("step-bar is-current"),
            "expected the current-step accent on the staircase, got: {html}"
        );
        assert_eq!(
            html.matches("step-bar").count(),
            1,
            "expected exactly one staircase bar for a one-step habit, got: {html}"
        );
    }

    #[test]
    fn an_unknown_habit_shows_a_quiet_fallback_with_a_link_back() {
        let html = render(RootAtUnknownHabit);

        assert!(
            !html.contains("step-bar"),
            "expected no staircase for a missing habit, got: {html}"
        );
        assert!(
            html.contains("Cette habitude") && html.contains("plus sur votre liste"),
            "expected the quiet not-found copy, got: {html}"
        );
        assert!(
            html.contains("Aujourd") && html.contains("quiet-link"),
            "expected a link back to Aujourd'hui, got: {html}"
        );
    }
}
