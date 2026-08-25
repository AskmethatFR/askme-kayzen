use crate::i18n::tr;
use dioxus::prelude::*;

#[component]
pub fn DataUnavailable() -> Element {
    rsx! {
        div { class: "screen",
            header { class: "masthead",
                span { class: "tag tag-accent", "Kaizen" }
            }
            h1 { class: "greeting", {tr!("data-unavailable-title")} }
            div { class: "empty-state",
                p { class: "lede", {tr!("data-unavailable-lede-1")} }
                p { class: "lede", {tr!("data-unavailable-lede-2")} }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::use_locale_for_tests;

    #[component]
    fn RootAtDataUnavailable() -> Element {
        use_locale_for_tests();
        rsx! {
            DataUnavailable {}
        }
    }

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    fn contains_word(haystack: &str, word: &str) -> bool {
        haystack
            .split(|c: char| !c.is_alphanumeric())
            .any(|token| token == word)
    }

    #[test]
    fn contains_word_ignores_a_match_embedded_inside_a_longer_word() {
        assert!(!contains_word("personne ne sait, ca sonne juste", "none"));
        assert!(!contains_word("nulle part ailleurs", "null"));
    }

    #[test]
    fn contains_word_still_finds_a_standalone_match() {
        assert!(contains_word("this value is none", "none"));
        assert!(contains_word("a null pointer", "null"));
    }

    // @scenario: persistence/S5
    #[test]
    fn the_screen_shows_a_calm_explanation_and_no_technical_wording() {
        let html = render(RootAtDataUnavailable);

        assert!(
            html.contains(r#"class="lede""#),
            "expected the explanation to use the app's own calm register, got: {html}"
        );
        let lowered = html.to_lowercase();
        for jargon in ["panic", "error", "exception", "stack", "null", "none"] {
            assert!(
                !contains_word(&lowered, jargon),
                "expected no technical wording ('{jargon}'), got: {html}"
            );
        }
    }

    // @scenario: persistence/S5
    #[test]
    fn the_screen_does_not_blame_disk_space_for_the_refusal() {
        let html = render(RootAtDataUnavailable);

        assert!(
            !html.to_lowercase().contains("espace"),
            "the refusal is never about disk space (see B7), got: {html}"
        );
    }
}
