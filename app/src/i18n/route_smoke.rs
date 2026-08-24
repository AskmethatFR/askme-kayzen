use crate::composition::Services;
use crate::i18n::use_locale_for_tests_as;
use crate::route::Route;
use dioxus::history::{MemoryHistory, provide_history_context};
use dioxus::prelude::*;
use dioxus_i18n::unic_langid::{LanguageIdentifier, langid};
use kayzen_core::habit_management::domain::goal::Goal;
use kayzen_core::habit_management::domain::habit::Habit;
use kayzen_core::habit_management::domain::habit_id::HabitId;
use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
use kayzen_core::habit_management::domain::habit_title::HabitTitle;
use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
use kayzen_core::shared::local_date::LocalDate;
use std::rc::Rc;

fn services_with_one_habit() -> Services {
    let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
    repository.save(&Habit::new(
        HabitId::new("route-smoke-1").unwrap(),
        HabitTitle::new("Route smoke habit".to_string()).unwrap(),
        Goal::new(5).unwrap(),
        LocalDate::from_epoch_day(20_000),
    ));
    Services::with_repository(repository)
}

#[component]
fn RouteSmokeRoot(path: &'static str, locale: LanguageIdentifier) -> Element {
    use_locale_for_tests_as(locale);
    use_hook(move || {
        provide_history_context(Rc::new(MemoryHistory::with_initial_path(path)));
    });
    use_context_provider(services_with_one_habit);
    rsx! {
        Router::<Route> {}
    }
}

fn render_route(path: &'static str, locale: LanguageIdentifier) -> String {
    let mut vdom = VirtualDom::new_with_props(RouteSmokeRoot, RouteSmokeRootProps { path, locale });
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

fn every_route() -> [&'static str; 7] {
    [
        "/",
        "/habit/route-smoke-1",
        "/habit/route-smoke-1/ritual",
        "/week",
        "/anchored",
        "/add",
        "/this-route-does-not-exist",
    ]
}

// @scenario: language/S1
// @scenario: language/S2
// @scenario: language/S3
#[test]
fn every_route_under_every_carried_locale_renders_with_no_fluent_or_bidi_residue() {
    for path in every_route() {
        for locale in [langid!("fr"), langid!("en")] {
            let html = render_route(path, locale.clone());

            assert!(
                !html.contains("{$") && !html.contains("{ $"),
                "expected no unresolved Fluent placeable in {path} under {locale}, got: {html}"
            );
            assert!(
                !html.chars().any(|c| matches!(c, '\u{2066}'..='\u{2069}')),
                "expected no bidi-isolate marks in {path} under {locale}, got: {html}"
            );
        }
    }
}

fn french_markers() -> [&'static str; 7] {
    [
        "Aujourd'hui",
        "habitude",
        "chaque jour",
        "pratique",
        "Reprendre",
        "Ajouter",
        "Désolé",
    ]
}

// @scenario: language/S4
#[test]
fn every_route_under_english_carries_no_french_marker() {
    for path in every_route() {
        let html = render_route(path, langid!("en"));

        for marker in french_markers() {
            assert!(
                !html.contains(marker),
                "expected no French marker {marker:?} in {path} under English, got: {html}"
            );
        }
    }
}
