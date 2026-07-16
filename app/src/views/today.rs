use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn Today() -> Element {
    let mut habits = use_signal(|| {
        vec![
            ("Lire", 4u32, false),
            ("Bouger un peu", 3, true),
            ("Respirer", 2, false),
        ]
    });

    rsx! {
        h1 { "Bonjour." }
        h4 { "Un seul petit pas suffit pour aujourd'hui." }
        for (i, (name, min, done)) in habits().into_iter().enumerate() {
            div { style: "display:flex; gap:16px; padding:20px 0;",
                div { flex: "1",
                    Link { to: Route::HabitDetail { id: i.to_string() }, "{name}" }
                    div { "aujourd'hui · {min} min" }
                }
                button {
                    onclick: move |_| habits.write()[i].2 = !done,
                    if done { "✓" } else { "○" }
                }
            }
        }

        div {
            Link { to: Route::AddHabit {}, "Ajouter une toute petite habitude" }
        }
    }
}
