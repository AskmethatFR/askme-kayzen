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

    fn link_body<'a>(nav: &'a str, href: &str) -> &'a str {
        let marker = format!(r#"<a href="{href}""#);
        let open = nav
            .find(&marker)
            .unwrap_or_else(|| panic!("expected a bar link to {href}, got: {nav}"));
        let after_open = &nav[open..];
        let close = after_open
            .find("</a>")
            .expect("expected the bar link to close");
        &after_open[..close]
    }

    // @scenario: bottom-nav/S1
    #[test]
    fn the_bar_shows_on_the_three_main_screens_with_its_destinations_in_order() {
        for path in ["/", "/week", "/anchored"] {
            let html = render_route(path, langid!("fr"));

            assert_eq!(
                html.matches(r#"<nav class="bottom-nav""#).count(),
                1,
                "expected exactly one bar on {path}, got: {html}"
            );
            let nav = nav_slice(&html);

            assert!(
                nav.contains(r#"aria-label="Navigation principale""#),
                "expected the bar to name itself for assistive technology on {path}, got: {nav}"
            );
            assert_eq!(
                nav.matches("<a ").count(),
                3,
                "expected exactly three destinations in the bar on {path}, got: {nav}"
            );
            assert_eq!(
                nav.matches(r#"aria-hidden="true""#).count(),
                3,
                "expected every destination icon hidden from assistive technology on {path}, got: {nav}"
            );
            assert_eq!(
                nav.matches(r#"class="bottom-nav-label""#).count(),
                3,
                "expected every destination to carry its label hook on {path}, got: {nav}"
            );

            for (href, visible) in [
                ("/", "Aujourd&#39;hui"),
                ("/week", "Semaine"),
                ("/anchored", "Ancrées"),
            ] {
                assert!(
                    link_body(nav, href).contains(&format!(">{visible}</span>")),
                    "expected the {href} destination to read {visible} on {path}, got: {nav}"
                );
            }

            let today = nav.find(">Aujourd&#39;hui</span>").unwrap_or_else(|| {
                panic!("expected the visible Aujourd'hui destination on {path}, got: {nav}")
            });
            let week = nav.find(">Semaine</span>").unwrap_or_else(|| {
                panic!("expected the visible Semaine destination on {path}, got: {nav}")
            });
            let anchored = nav.find(">Ancrées</span>").unwrap_or_else(|| {
                panic!("expected the visible Ancrées destination on {path}, got: {nav}")
            });
            assert!(
                today < week && week < anchored,
                "expected the visible destinations ordered Aujourd'hui, Semaine, Ancrées on {path}, got: {nav}"
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

    // @scenario: bottom-nav/S2
    #[test]
    fn the_current_destination_is_marked_once_and_matches_the_route() {
        for (path, href, visible) in [
            ("/", "/", "Aujourd&#39;hui"),
            ("/week", "/week", "Semaine"),
            ("/anchored", "/anchored", "Ancrées"),
        ] {
            let html = render_route(path, langid!("fr"));
            let nav = nav_slice(&html);

            assert_eq!(
                nav.matches(r#"aria-current="page""#).count(),
                1,
                "expected exactly one current-page marker on {path}, got: {nav}"
            );
            let tag = current_page_tag(nav);
            assert!(
                tag.contains(&format!(r#"href="{href}""#)),
                "expected the current-page marker on {path} to sit on the {href} link, got: {tag}"
            );
            assert!(
                tag.contains(&format!(r#"aria-label="{visible} · navigation""#)),
                "expected the current-page link on {path} to announce {visible} then navigation, got: {tag}"
            );
        }
    }

    // @scenario: bottom-nav/S4
    #[test]
    fn focus_screens_keep_the_whole_screen_to_themselves() {
        for path in [
            "/habit/route-smoke-1",
            "/habit/route-smoke-1/ritual",
            "/paused",
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

        for (href, visible) in [("/", "Today"), ("/week", "Week"), ("/anchored", "Anchored")] {
            assert!(
                link_body(nav, href).contains(&format!(">{visible}</span>")),
                "expected the {href} destination to read {visible} under English, got: {nav}"
            );
        }

        let today = nav
            .find(">Today</span>")
            .unwrap_or_else(|| panic!("expected the visible Today destination, got: {nav}"));
        let week = nav
            .find(">Week</span>")
            .unwrap_or_else(|| panic!("expected the visible Week destination, got: {nav}"));
        let anchored = nav
            .find(">Anchored</span>")
            .unwrap_or_else(|| panic!("expected the visible Anchored destination, got: {nav}"));
        assert!(
            today < week && week < anchored,
            "expected the visible English destinations ordered Today, Week, Anchored, got: {nav}"
        );
        for marker in ["Aujourd", "Semaine", "Ancrées"] {
            assert!(
                !nav.contains(marker),
                "expected no French destination {marker:?} under English, got: {nav}"
            );
        }
    }
}
