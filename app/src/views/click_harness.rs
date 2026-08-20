use dioxus::prelude::*;
use dioxus_core::{AttributeValue, ElementId, Mutation, Mutations, NoOpMutations};
use dioxus_html::{
    PlatformEventData, SerializedAnimationData, SerializedHtmlEventConverter, SerializedMouseData,
};
use std::any::Any;
use std::rc::Rc;
use std::sync::Once;

static INSTALL_EVENT_CONVERTER: Once = Once::new();

// Locates by aria-label (clicks) or by registered listener name (synthetic
// DOM events with no visible/accessible target, e.g. an animation tick)
// against the Mutations captured at `open()`'s first render only:
// `ElementId`s are reassigned on every diff, so that lookup stays valid
// before any dispatch runs — a second dispatch on the same `Screen` panics
// rather than silently targeting a stale id.
pub(crate) struct Screen {
    vdom: VirtualDom,
    first_render: Mutations,
    dispatched: bool,
}

impl Screen {
    pub(crate) fn open(root: fn() -> Element) -> Self {
        INSTALL_EVENT_CONVERTER.call_once(|| {
            dioxus::html::set_event_converter(Box::new(SerializedHtmlEventConverter))
        });
        let mut vdom = VirtualDom::new(root);
        let first_render = vdom.rebuild_to_vec();
        Screen {
            vdom,
            first_render,
            dispatched: false,
        }
    }

    pub(crate) fn html(&self) -> String {
        dioxus_ssr::render(&self.vdom)
    }

    pub(crate) fn click(&mut self, aria_label: &str) {
        let id = self.locate_by_aria_label(aria_label);
        let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(
            SerializedMouseData::default(),
        )));
        self.dispatch("click", id, data, true);
    }

    pub(crate) fn fire_animation_iteration(&mut self, listener_name: &str) {
        let id = self.locate_by_listener(listener_name);
        let animation_data: SerializedAnimationData = serde_json::from_value(serde_json::json!({
            "animation_name": "",
            "pseudo_element": "",
            "elapsed_time": 0.0,
        }))
        .expect("a well-formed synthetic animation payload");
        let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(animation_data)));
        self.dispatch(listener_name, id, data, true);
    }

    fn dispatch(&mut self, event_name: &str, id: ElementId, data: Rc<dyn Any>, bubbles: bool) {
        assert!(
            !self.dispatched,
            "a Screen supports one dispatch: ElementIds move after the diff"
        );
        self.vdom
            .runtime()
            .handle_event(event_name, dioxus_core::Event::new(data, bubbles), id);
        self.vdom.render_immediate(&mut NoOpMutations);
        self.dispatched = true;
    }

    fn locate_by_aria_label(&self, aria_label: &str) -> ElementId {
        let mut matches = Vec::new();
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
                    matches.push(*id);
                }
                available.push(text.clone());
            }
        }
        match matches.as_slice() {
            [id] => *id,
            [] => {
                panic!(
                    "no element with aria-label {aria_label:?} found; available labels: {available:?}"
                )
            }
            _ => panic!(
                "ambiguous aria-label {aria_label:?}: {} elements share it",
                matches.len()
            ),
        }
    }

    fn locate_by_listener(&self, event_name: &str) -> ElementId {
        let mut matches = Vec::new();
        let mut available = Vec::new();
        for edit in &self.first_render.edits {
            if let Mutation::NewEventListener { name, id } = edit {
                if name == event_name {
                    matches.push(*id);
                }
                available.push(name.clone());
            }
        }
        match matches.as_slice() {
            [id] => *id,
            [] => {
                panic!("no listener named {event_name:?} found; available listeners: {available:?}")
            }
            _ => panic!(
                "ambiguous listener {event_name:?}: {} elements share it",
                matches.len()
            ),
        }
    }
}

// Test List — Screen::fire_animation_iteration:
// - dispatches to the element registered for the named listener, running its handler
// - no element listens for the named event -> panics naming what IS available
// - two elements listen for the same named event -> panics as ambiguous
// - a second dispatch on the same Screen panics with the shared "one dispatch"
//   message; the guard lives in the shared dispatch() so click-then-fire is
//   covered by construction, and click-then-click / fire-then-fire below each
//   pin one concrete instance of it
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
    #[should_panic(expected = "a Screen supports one dispatch")]
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

    #[test]
    #[should_panic(expected = "no element with aria-label")]
    fn clicking_an_aria_label_nobody_has_panics_instead_of_silently_doing_nothing() {
        let mut screen = Screen::open(RootWithVanishingButton);
        screen.click("nowhere to be found");
    }

    #[component]
    fn RootWithAnimationListener() -> Element {
        let mut ticks = use_signal(|| 0);
        rsx! {
            div {
                class: "tick",
                onanimationiteration: move |_| ticks += 1,
            }
            p { "ticks: {ticks}" }
        }
    }

    #[test]
    fn firing_animationiteration_dispatches_to_the_element_registered_for_it() {
        let mut screen = Screen::open(RootWithAnimationListener);

        screen.fire_animation_iteration("animationiteration");

        assert!(
            screen.html().contains("ticks: 1"),
            "expected the listener to have run, got: {}",
            screen.html()
        );
    }

    #[test]
    #[should_panic(expected = "no listener named")]
    fn firing_an_event_nobody_listens_for_panics_instead_of_silently_doing_nothing() {
        let mut screen = Screen::open(RootWithAnimationListener);
        screen.fire_animation_iteration("animationend");
    }

    #[component]
    fn RootWithTwoAnimationListeners() -> Element {
        rsx! {
            div { onanimationiteration: move |_| {} }
            div { onanimationiteration: move |_| {} }
        }
    }

    #[test]
    #[should_panic(expected = "ambiguous listener")]
    fn firing_an_event_two_elements_listen_for_panics_instead_of_silently_picking_the_first() {
        let mut screen = Screen::open(RootWithTwoAnimationListeners);
        screen.fire_animation_iteration("animationiteration");
    }

    #[test]
    #[should_panic(expected = "a Screen supports one dispatch")]
    fn firing_twice_on_the_same_screen_panics_instead_of_silently_targeting_a_stale_id() {
        let mut screen = Screen::open(RootWithAnimationListener);
        screen.fire_animation_iteration("animationiteration");
        screen.fire_animation_iteration("animationiteration");
    }
}
