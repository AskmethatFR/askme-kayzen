use dioxus::prelude::*;

mod composition;
mod infrastructure;
mod route;
mod views;

use composition::Services;
use route::Route;
use views::DataUnavailable;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Dependency injection, Dioxus-style: provide the composition root once at the
    // top of the tree; every child screen reads it with `use_context::<Services>()`.
    let services = use_hook(Services::new);

    match services {
        Some(services) => {
            use_context_provider(move || services.clone());
            rsx! {
                document::Link { rel: "icon", href: FAVICON }
                document::Link { rel: "stylesheet", href: MAIN_CSS }
                Router::<Route> {}
            }
        }
        None => rsx! {
            document::Link { rel: "icon", href: FAVICON }
            document::Link { rel: "stylesheet", href: MAIN_CSS }
            DataUnavailable {}
        },
    }
}
