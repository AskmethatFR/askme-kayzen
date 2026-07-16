use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn Ritual(id: String) -> Element {
    rsx! {
        h1 { "Rituel" }
        Link { to: Route::HabitDetail { id }, "Détail" }
    }
}
