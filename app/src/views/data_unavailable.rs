use dioxus::prelude::*;

#[component]
pub fn DataUnavailable() -> Element {
    rsx! {
        div { class: "screen",
            header { class: "masthead",
                span { class: "tag tag-accent", "Kaizen" }
            }
            h1 { class: "greeting", "Un instant." }
            div { class: "empty-state",
                p { class: "lede", "Impossible de trouver un endroit sûr où garder vos habitudes sur cet appareil." }
                p { class: "lede", "Rien n'a été écrit. Réessayez une fois de l'espace disponible pour l'application." }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(root: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_in_place();
        dioxus_ssr::render(&vdom)
    }

    // @scenario: persistence/S5
    #[test]
    fn the_screen_shows_a_calm_explanation_and_no_technical_wording() {
        let html = render(DataUnavailable);

        assert!(
            html.contains(r#"class="lede""#),
            "expected the explanation to use the app's own calm register, got: {html}"
        );
        let lowered = html.to_lowercase();
        for jargon in ["panic", "error", "exception", "stack", "null", "none"] {
            assert!(
                !lowered.contains(jargon),
                "expected no technical wording ('{jargon}'), got: {html}"
            );
        }
    }
}
