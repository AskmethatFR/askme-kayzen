use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn HabitDetail(id: String) -> Element {
    rsx! {
        h1 { "Détail" }
        Link { to: Route::Ritual { id: id.clone() }, "Faire ma minute" }
        Link { to: Route::Today {}, "Aujourd'hui" }
    }
}
