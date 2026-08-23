use dioxus::prelude::*;

mod composition;
mod infrastructure;
mod route;
mod views;

use composition::Services;
use route::Route;
use views::DataUnavailable;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Dependency injection, Dioxus-style: provide the composition root once at the
    // top of the tree; every child screen reads it with `use_context::<Services>()`.
    let services = use_hook(Services::new);
    app_shell(services)
}

// @law: Dioxus requires a component instance to call the same hooks in the
// same order on every render. `use_context_provider` below sits inside one
// match arm, which stays sound only because `App`'s `use_hook(Services::new)`
// resolves once and fixes this branch for the whole lifetime of the
// component -- a future edit that lets the branch vary across renders of the
// same instance would break hook ordering and panic in release builds.
fn app_shell(services: Option<Services>) -> Element {
    let content = match services {
        Some(services) => {
            use_context_provider(move || services.clone());
            rsx! {
                Router::<Route> {}
            }
        }
        None => rsx! {
            DataUnavailable {}
        },
    };
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        {content}
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;

    use super::*;

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    #[component]
    fn AppWithAvailableServices() -> Element {
        app_shell(Some(Services::with_repository(Rc::new(
            InMemoryHabitRepository::new(),
        ))))
    }

    #[component]
    fn AppWithUnavailableServices() -> Element {
        app_shell(None)
    }

    // @scenario: persistence/S5
    #[test]
    fn when_services_are_available_the_router_is_rendered_not_the_refusal_screen() {
        let html = render(AppWithAvailableServices);

        assert!(
            html.contains("masthead-date"),
            "expected the Today screen via the Router, got: {html}"
        );
    }

    // @scenario: persistence/S5
    #[test]
    fn when_services_are_unavailable_the_refusal_screen_is_rendered_not_the_router() {
        let html = render(AppWithUnavailableServices);

        assert!(
            html.contains("Désolé"),
            "expected the DataUnavailable screen, got: {html}"
        );
        assert!(
            !html.contains("masthead-date"),
            "expected the Router/Today screen NOT to be rendered, got: {html}"
        );
    }
}
