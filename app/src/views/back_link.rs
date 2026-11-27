use crate::i18n::tr;
use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn BackLink() -> Element {
    rsx! {
        Link {
            class: "detail-back",
            to: Route::Today {},
            aria_label: tr!("detail-back-to-today"),
            svg {
                class: "detail-back-icon",
                view_box: "0 0 24 24",
                "aria-hidden": "true",
                "focusable": "false",
                path { d: "M15 5l-7 7 7 7" }
            }
        }
    }
}
