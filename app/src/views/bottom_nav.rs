use crate::i18n::tr;
use crate::route::Route;
use dioxus::prelude::*;

#[component]
pub fn BottomNav() -> Element {
    rsx! {
        nav { class: "bottom-nav", aria_label: tr!("bottom-nav-aria"),
            Link {
                to: Route::Today {},
                aria_label: tr!("bottom-nav-today-aria"),
                svg {
                    class: "bottom-nav-icon",
                    view_box: "0 0 24 24",
                    "aria-hidden": "true",
                    "focusable": "false",
                    circle { cx: "12", cy: "12", r: "4" }
                    path { d: "M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4" }
                }
                span { class: "bottom-nav-label", {tr!("bottom-nav-today")} }
            }
            Link {
                to: Route::Week {},
                aria_label: tr!("bottom-nav-week-aria"),
                svg {
                    class: "bottom-nav-icon",
                    view_box: "0 0 24 24",
                    "aria-hidden": "true",
                    "focusable": "false",
                    path { d: "M4 18c4 0 5-4 8-4s4-6 8-8" }
                }
                span { class: "bottom-nav-label", {tr!("bottom-nav-week")} }
            }
            Link {
                to: Route::Anchored {},
                aria_label: tr!("bottom-nav-anchored-aria"),
                svg {
                    class: "bottom-nav-icon",
                    view_box: "0 0 24 24",
                    "aria-hidden": "true",
                    "focusable": "false",
                    ellipse { cx: "12", cy: "15", rx: "7", ry: "4.5" }
                    path { d: "M12 10.5V4" }
                }
                span { class: "bottom-nav-label", {tr!("bottom-nav-anchored")} }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::composition::Services;
    use crate::i18n::route_smoke::render_route;
    use crate::i18n::use_locale_for_tests;
    use crate::route::Route;
    use crate::views::click_harness::Screen;
    use dioxus::prelude::*;
    use dioxus_i18n::unic_langid::langid;
    use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use std::rc::Rc;

    #[component]
    fn RootAtTodayForBottomNav() -> Element {
        use_locale_for_tests();
        use_context_provider(|| Services::with_repository(Rc::new(InMemoryHabitRepository::new())));
        rsx! {
            Router::<Route> {}
        }
    }

    fn nav_slice(html: &str) -> &str {
        let start = html
            .find(r#"<nav class="bottom-nav""#)
            .expect("expected a bottom navigation bar in the rendered page");
        let after_open = &html[start..];
        let end = after_open
            .find("</nav>")
            .expect("expected the bottom navigation bar to close");
        &after_open[..end]
    }

    fn current_page_tag(nav: &str) -> &str {
        let marker = nav
            .find(r#"aria-current="page""#)
            .expect("expected a current-page marker in the bar");
        let open = nav[..marker]
            .rfind("<a ")
            .expect("expected the current-page marker to sit on a link");
        let close = nav[marker..]
            .find('>')
            .expect("expected the current-page link tag to close");
        &nav[open..marker + close + 1]
    }

    // @scenario: bottom-nav/S1
    #[test]
    fn the_bar_shows_on_the_three_main_screens_with_its_destinations_in_order() {
        for path in ["/", "/week", "/anchored"] {
            let html = render_route(path, langid!("fr"));
            let nav = nav_slice(&html);

            assert_eq!(
                nav.matches("<a ").count(),
                3,
                "expected exactly three destinations in the bar on {path}, got: {nav}"
            );
            let today = nav.find("Aujourd&#39;hui").unwrap_or_else(|| {
                panic!("expected the Aujourd'hui destination on {path}, got: {nav}")
            });
            let week = nav.find("Semaine").unwrap_or_else(|| {
                panic!("expected the Semaine destination on {path}, got: {nav}")
            });
            let anchored = nav.find("Ancrées").unwrap_or_else(|| {
                panic!("expected the Ancrées destination on {path}, got: {nav}")
            });
            assert!(
                today < week && week < anchored,
                "expected the destinations ordered Aujourd'hui, Semaine, Ancrées on {path}, got: {nav}"
            );
        }
    }

    // @scenario: bottom-nav/S3
    #[test]
    fn tapping_ancrees_in_the_bar_opens_the_ancrees_screen() {
        let mut screen = Screen::open(RootAtTodayForBottomNav);
        assert!(
            screen.html().contains("Bonjour."),
            "expected to start on Aujourd'hui, got: {}",
            screen.html()
        );

        screen.click("Ancrées · navigation");

        let html = screen.html();
        assert!(
            html.contains("Vous suivez"),
            "expected the Ancrées screen after tapping the bar, got: {html}"
        );
        assert!(
            !html.contains("Bonjour."),
            "expected to have left Aujourd'hui after tapping the bar, got: {html}"
        );
        let tag = current_page_tag(nav_slice(&html));
        assert!(
            tag.contains(r#"href="/anchored""#),
            "expected the bar to mark Ancrées as the current page, got: {tag}"
        );
    }

    // @scenario: bottom-nav/S4
    #[test]
    fn focus_screens_keep_the_whole_screen_to_themselves() {
        for path in [
            "/habit/route-smoke-1",
            "/habit/route-smoke-1/ritual",
            "/add",
        ] {
            let html = render_route(path, langid!("fr"));

            assert!(
                html.contains(r#"class="screen"#),
                "expected {path} to render its own screen, got: {html}"
            );
            assert!(
                !html.contains(r#"class="bottom-nav""#),
                "expected no navigation bar on {path}, got: {html}"
            );
        }
    }

    // @scenario: bottom-nav/S5
    #[test]
    fn the_bar_speaks_english_under_an_english_locale() {
        let html = render_route("/", langid!("en"));
        let nav = nav_slice(&html);

        let today = nav
            .find("Today")
            .unwrap_or_else(|| panic!("expected the Today destination under English, got: {nav}"));
        let week = nav
            .find("Week")
            .unwrap_or_else(|| panic!("expected the Week destination under English, got: {nav}"));
        let anchored = nav.find("Anchored").unwrap_or_else(|| {
            panic!("expected the Anchored destination under English, got: {nav}")
        });
        assert!(
            today < week && week < anchored,
            "expected the English destinations ordered Today, Week, Anchored, got: {nav}"
        );
        for marker in ["Aujourd", "Semaine", "Ancrées"] {
            assert!(
                !nav.contains(marker),
                "expected no French destination {marker:?} under English, got: {nav}"
            );
        }
    }
}
