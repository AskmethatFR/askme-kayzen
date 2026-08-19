use dioxus::prelude::*;
use dioxus_core::{AttributeValue, Mutation, Mutations, NoOpMutations};
use dioxus_html::{PlatformEventData, SerializedHtmlEventConverter, SerializedMouseData};
use std::any::Any;
use std::rc::Rc;
use std::sync::Once;

static INSTALL_EVENT_CONVERTER: Once = Once::new();

// Locates by aria-label against the Mutations captured at `open()`'s first
// render only: `ElementId`s are reassigned on every diff, so that lookup
// stays valid before any `click()` runs, and a second click on the same
// `Screen` is not supported.
pub(crate) struct Screen {
    vdom: VirtualDom,
    first_render: Mutations,
}

impl Screen {
    pub(crate) fn open(root: fn() -> Element) -> Self {
        INSTALL_EVENT_CONVERTER.call_once(|| {
            dioxus::html::set_event_converter(Box::new(SerializedHtmlEventConverter))
        });
        let mut vdom = VirtualDom::new(root);
        let first_render = vdom.rebuild_to_vec();
        Screen { vdom, first_render }
    }

    pub(crate) fn html(&self) -> String {
        dioxus_ssr::render(&self.vdom)
    }

    pub(crate) fn click(&mut self, aria_label: &str) {
        let id = self.locate(aria_label);
        let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(
            SerializedMouseData::default(),
        )));
        self.vdom
            .runtime()
            .handle_event("click", dioxus_core::Event::new(data, true), id);
        self.vdom.render_immediate(&mut NoOpMutations);
    }

    fn locate(&self, aria_label: &str) -> dioxus_core::ElementId {
        let mut available = Vec::new();
        for edit in &self.first_render.edits {
            if let Mutation::SetAttribute {
                name: "aria-label",
                value: AttributeValue::Text(text),
                id,
                ..
            } = edit
            {
                if text == aria_label {
                    return *id;
                }
                available.push(text.clone());
            }
        }
        panic!("no element with aria-label {aria_label:?} found; available labels: {available:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn RootWithVanishingButton() -> Element {
        let mut visible = use_signal(|| true);
        let label = "vanish";
        rsx! {
            if visible() {
                button {
                    aria_label: "{label}",
                    onclick: move |_| visible.set(false),
                    "x"
                }
            }
        }
    }

    #[test]
    #[should_panic(expected = "a Screen supports one click")]
    fn clicking_twice_on_the_same_screen_panics_instead_of_silently_targeting_a_stale_id() {
        let mut screen = Screen::open(RootWithVanishingButton);
        screen.click("vanish");
        screen.click("vanish");
    }

    #[component]
    fn RootWithDuplicateAriaLabels() -> Element {
        let label = "dup";
        rsx! {
            button { aria_label: "{label}", "a" }
            button { aria_label: "{label}", "b" }
        }
    }

    #[test]
    #[should_panic(expected = "ambiguous aria-label")]
    fn locating_a_duplicated_aria_label_panics_instead_of_silently_picking_the_first() {
        let mut screen = Screen::open(RootWithDuplicateAriaLabels);
        screen.click("dup");
    }
}
