use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn Anchored() -> Element {
    rsx! {
        h1 { "Ancrées" }
        Link { to: Route::Today {}, "Aujourd'hui" }
    }
}
