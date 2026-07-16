use dioxus::prelude::*;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut habits = use_signal(|| {
        vec![
            ("Lire", 4u32, false),
            ("Bouger un peu", 3, true),
            ("Respirer", 2, false),
        ]
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        h1 { "Bonjour." }
        for (i, (name, min, done)) in habits().into_iter().enumerate() {
            div { style: "display:flex; gap:16px; padding:20px 0;",
                div { flex: "1",
                    div { "{name}" }
                    div { "aujourd'hui · {min} min" }
                }
                button {
                    onclick: move |_| habits.write()[i].2 = !done,
                    if done { "✓" } else { "○" }
                }
            }
        }
    }
}
