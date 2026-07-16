use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        h1 { "Cette page n'existe pas." }
        Link { to: Route::Today {}, "Aujourd'hui" }
    }
}
