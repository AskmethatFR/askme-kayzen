use super::bottom_nav::BottomNav;
use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn MainScreenFrame() -> Element {
    rsx! {
        div { class: "main-screen-frame",
            Outlet::<Route> {}
            BottomNav {}
        }
    }
}
