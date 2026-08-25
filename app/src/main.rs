use dioxus::prelude::*;

mod composition;
mod i18n;
mod infrastructure;
mod route;
mod views;

use composition::Services;
use dioxus_i18n::prelude::use_init_i18n;
use route::Route;
use views::DataUnavailable;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const VIEWPORT_CONTENT: &str = "width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover";

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    app_root(Services::new)
}

/// `App`'s body, with where the services come from taken as a parameter — the
/// seam a test needs to prove `use_init_i18n(i18n::config)` really is wired in
/// without also calling the real `Services::new()`, which touches the user's
/// actual data directory.
fn app_root(services_fn: impl FnOnce() -> Option<Services> + 'static) -> Element {
    use_init_i18n(i18n::config);
    // Dependency injection, Dioxus-style: provide the composition root once at the
    // top of the tree; every child screen reads it with `use_context::<Services>()`.
    let services = use_hook(services_fn);
    app_shell(services)
}

// @law: Dioxus requires a component instance to call the same hooks in the
// same order on every render. `use_context_provider` below sits inside one
// match arm, which stays sound only because `app_root`'s `use_hook(services_fn)`
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
        document::Meta { name: "viewport", content: VIEWPORT_CONTENT }
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        {content}
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use dioxus::document::{Document, Eval};
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;

    use super::*;

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    struct RecordedHeadElement {
        name: String,
        attributes: Vec<(String, String)>,
    }

    #[derive(Default)]
    struct HeadElementSpy {
        head_elements: RefCell<Vec<RecordedHeadElement>>,
    }

    impl Document for HeadElementSpy {
        fn eval(&self, _js: String) -> Eval {
            unimplemented!("this spy overrides create_head_element, so eval is never reached")
        }

        fn create_head_element(
            &self,
            name: &str,
            attributes: &[(&str, String)],
            _contents: Option<String>,
        ) {
            self.head_elements.borrow_mut().push(RecordedHeadElement {
                name: name.to_string(),
                attributes: attributes
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.clone()))
                    .collect(),
            });
        }
    }

    // @law: `VirtualDom::new` takes a bare `fn() -> Element` — no captures
    // allowed — so `AppShellWithHeadSpy` cannot receive the spy as a closure
    // capture. It travels through this thread-local instead, same shape as
    // `ritual.rs`'s `SHARED_REPOSITORY`. Scoped to this test module only;
    // production code never reads it. Typed as `Rc<dyn Document>` (the port),
    // not `Rc<HeadElementSpy>` (one implementation of it) — the thread-local's
    // contract is "hand a `Document` to the component".
    thread_local! {
        static SHARED_HEAD_SPY: RefCell<Option<Rc<dyn Document>>> =
            const { RefCell::new(None) };
    }

    struct SharedHeadSpyGuard;

    impl Drop for SharedHeadSpyGuard {
        fn drop(&mut self) {
            SHARED_HEAD_SPY.with(|cell| *cell.borrow_mut() = None);
        }
    }

    #[component]
    fn AppShellWithHeadSpy() -> Element {
        crate::i18n::use_locale_for_tests();
        let document = SHARED_HEAD_SPY.with(|cell| {
            cell.borrow()
                .clone()
                .expect("a head spy was seeded before rendering")
        });
        use_context_provider(move || document.clone());
        app_shell(Some(Services::with_repository(Rc::new(
            InMemoryHabitRepository::new(),
        ))))
    }

    #[component]
    fn AppWithAvailableServices() -> Element {
        crate::i18n::use_locale_for_tests();
        app_shell(Some(Services::with_repository(Rc::new(
            InMemoryHabitRepository::new(),
        ))))
    }

    #[component]
    fn AppWithUnavailableServices() -> Element {
        crate::i18n::use_locale_for_tests();
        app_shell(None)
    }

    #[component]
    fn AppRootWithInMemoryServicesAndTheRealDeviceLocale() -> Element {
        app_root(|| {
            Some(Services::with_repository(Rc::new(
                InMemoryHabitRepository::new(),
            )))
        })
    }

    #[test]
    fn app_wires_the_real_i18n_config_so_the_masthead_renders_a_real_translation() {
        let html = render(AppRootWithInMemoryServicesAndTheRealDeviceLocale);

        assert!(
            html.contains("Aujourd&#39;hui") || html.contains("Today"),
            "expected the production i18n config to translate the Today masthead \
             into either supported catalogue, got: {html}"
        );
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

    #[test]
    fn app_shell_registers_a_viewport_meta_that_opts_into_safe_area_insets() {
        let spy = Rc::new(HeadElementSpy::default());
        SHARED_HEAD_SPY.with(|cell| *cell.borrow_mut() = Some(spy.clone() as Rc<dyn Document>));
        let _guard = SharedHeadSpyGuard;

        // Seeded over `Some(services)` — the live-session path every real
        // launch takes — not `None`/`DataUnavailable`, so this cannot pass
        // by luck of which branch happens to carry the meta.
        let mut vdom = VirtualDom::new(AppShellWithHeadSpy);
        vdom.rebuild_in_place();

        let viewport_meta = spy
            .head_elements
            .borrow()
            .iter()
            .find(|element| {
                element.name == "meta"
                    && element
                        .attributes
                        .iter()
                        .any(|(key, value)| key == "name" && value == "viewport")
            })
            .map(|element| element.attributes.clone());

        let viewport_meta = viewport_meta.expect(
            "expected app_shell to register a <meta name=\"viewport\"> head element \
             when services are available, got none",
        );
        let content = viewport_meta
            .iter()
            .find(|(key, _)| key == "content")
            .map(|(_, value)| value.clone())
            .expect("expected the viewport meta to carry a content attribute");

        // This runtime tag supersedes the dx-generated shell's viewport tag
        // wholesale (Chrome/Safari/Firefox all take the last viewport tag as
        // a unit, not a per-key merge) — so every key the shell's tag would
        // otherwise have provided must be present here, or the device falls
        // back to the 980px desktop-width layout viewport. Each key is
        // pinned individually so trimming any one of them fails the test.
        for key in [
            "width=device-width",
            "initial-scale=1.0",
            "maximum-scale=1.0",
            "user-scalable=no",
            "viewport-fit=cover",
        ] {
            assert!(
                content.contains(key),
                "expected the viewport meta's content to carry {key}, got: {content}"
            );
        }
    }
}
