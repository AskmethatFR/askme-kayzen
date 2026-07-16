use dioxus::prelude::*;

#[component]
pub fn Today() -> Element {
    rsx! { "today" }
}

#[component]
pub fn HabitDetail(id: String) -> Element {
    rsx! { "detail {id}" }
}

#[component]
pub fn Ritual(id: String) -> Element {
    rsx! { "ritual {id}" }
}

#[component]
pub fn Week() -> Element {
    rsx! { "week" }
}

#[component]
pub fn Anchored() -> Element {
    rsx! { "anchored" }
}

#[component]
pub fn AddHabit() -> Element {
    rsx! { "add" }
}

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    rsx! { "not found {segments:?}" }
}
