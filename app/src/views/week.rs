use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn Week() -> Element {
    rsx! {
        h1 { "Cette semaine" }
        Link { to: Route::Today {}, "Aujourd'hui" }
    }
}
