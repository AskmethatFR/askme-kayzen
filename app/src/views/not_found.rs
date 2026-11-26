use crate::i18n::tr;
use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        div { class: "screen",
            h1 { class: "greeting", {tr!("not-found-title")} }
            Link { class: "quiet-link", to: Route::Today {}, {tr!("habit-not-found-back-link")} }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{use_locale_for_tests, use_locale_for_tests_as};
    use dioxus::history::{MemoryHistory, provide_history_context};
    use dioxus_i18n::unic_langid::langid;
    use std::rc::Rc;

    #[component]
    fn RootWithFrenchLocale() -> Element {
        use_locale_for_tests();
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/does-not-exist")));
        });
        rsx! {
            Router::<crate::route::Route> {}
        }
    }

    #[component]
    fn RootWithEnglishLocale() -> Element {
        use_locale_for_tests_as(langid!("en"));
        use_hook(|| {
            provide_history_context(Rc::new(MemoryHistory::with_initial_path("/does-not-exist")));
        });
        rsx! {
            Router::<crate::route::Route> {}
        }
    }

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    // @scenario: language/S1
    #[test]
    fn an_english_locale_renders_the_not_found_screen_in_english() {
        let html = render(RootWithEnglishLocale);

        assert!(
            html.contains("This page doesn&#39;t exist."),
            "expected the title in English, got: {html}"
        );
        assert!(
            html.contains(">Back to Today<"),
            "expected the back-to-today link in English, got: {html}"
        );
    }

    // @scenario: language/S1
    #[test]
    fn the_not_found_screen_uses_the_shared_screen_shell() {
        let html = render(RootWithFrenchLocale);

        assert!(
            html.contains(r#"class="screen""#),
            "expected the page inside the shared .screen shell, got: {html}"
        );
        assert!(
            html.contains(r#"<h1 class="greeting">"#),
            "expected the title as h1.greeting, got: {html}"
        );
        assert!(
            html.contains(r#"class="quiet-link""#),
            "expected the back link to be a quiet-link, got: {html}"
        );
    }

    #[test]
    fn a_french_locale_renders_the_not_found_screen_unchanged() {
        let html = render(RootWithFrenchLocale);

        assert!(
            html.contains("Cette page n&#39;existe pas."),
            "expected the byte-identical French title, got: {html}"
        );
        assert!(
            html.contains(">Retour à Aujourd&#39;hui<"),
            "expected the French back-to-today link with the shared label, got: {html}"
        );
    }
}
