use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn AddHabit() -> Element {
    rsx! {
        h1 { "Ajouter" }
        Link { to: Route::Today {}, "Aujourd'hui" }
    }
}
