use crate::i18n::tr;
use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        h1 { {tr!("not-found-title")} }
        Link { to: Route::Today {}, {tr!("not-found-today-link")} }
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
            html.contains(">Today<"),
            "expected the back-to-today link in English, got: {html}"
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
            html.contains(">Aujourd&#39;hui<"),
            "expected the byte-identical French back-to-today link, got: {html}"
        );
    }
}
