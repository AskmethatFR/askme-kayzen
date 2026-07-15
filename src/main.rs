use dioxus::prelude::*;

mod habit_management;
pub mod shared;

fn main() {
    launch(app);
}
fn app() -> Result<VNode, dioxus::prelude::RenderError> {
    let mut habits = use_signal(|| {
        vec![
            ("Lire", 4u32, false),
            ("Bouger un peu", 3, true),
            ("Respirer", 2, false),
        ]
    });

    rsx! {
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
