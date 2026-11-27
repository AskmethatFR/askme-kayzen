use crate::back_key::BackKey;
use crate::views::*;
use dioxus::prelude::*;

#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(BackKey)]
        #[layout(MainScreenFrame)]
            #[route("/")]
            Today {},
            #[route("/week")]
            Week {},
            #[route("/anchored")]
            Anchored {},
        #[end_layout]
        #[route("/paused")]
        Paused {},
        #[route("/habit/:id/ritual")]
        Ritual { id: String },
        #[route("/habit/:id")]
        HabitDetail { id: String },
        #[route("/add")]
        AddHabit {},
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}

pub fn back_target(route: &Route) -> Option<Route> {
    match route {
        Route::Today {} => None,
        Route::Ritual { id } => Some(Route::HabitDetail { id: id.clone() }),
        Route::Week {}
        | Route::Anchored {}
        | Route::Paused {}
        | Route::HabitDetail { .. }
        | Route::AddHabit {}
        | Route::NotFound { .. } => Some(Route::Today {}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // @scenario: system-back/S1
    #[test]
    fn back_target_sends_every_screen_but_today_and_a_ritual_up_to_today() {
        for route in [
            Route::Week {},
            Route::Anchored {},
            Route::Paused {},
            Route::HabitDetail {
                id: "abc".to_string(),
            },
            Route::AddHabit {},
            Route::NotFound {
                segments: vec!["nope".to_string()],
            },
        ] {
            assert_eq!(
                back_target(&route),
                Some(Route::Today {}),
                "Back on {route} must go up to Today"
            );
        }
    }

    // @scenario: system-back/S1
    #[test]
    fn back_target_sends_a_ritual_up_to_its_habit_detail() {
        let ritual = Route::Ritual {
            id: "abc".to_string(),
        };

        assert_eq!(
            back_target(&ritual),
            Some(Route::HabitDetail {
                id: "abc".to_string()
            })
        );
    }

    // @scenario: system-back/S2
    #[test]
    fn back_target_finds_no_level_above_today() {
        assert_eq!(back_target(&Route::Today {}), None);
    }

    fn named_routes() -> Vec<(Route, &'static str)> {
        vec![
            (Route::Today {}, "/"),
            (
                Route::HabitDetail {
                    id: "abc".to_string(),
                },
                "/habit/abc",
            ),
            (
                Route::Ritual {
                    id: "abc".to_string(),
                },
                "/habit/abc/ritual",
            ),
            (Route::Week {}, "/week"),
            (Route::Anchored {}, "/anchored"),
            (Route::Paused {}, "/paused"),
            (Route::AddHabit {}, "/add"),
        ]
    }

    #[test]
    fn display_renders_exact_path_per_variant() {
        for (route, expected_path) in named_routes() {
            assert_eq!(route.to_string(), expected_path);
        }
    }

    #[test]
    fn parse_round_trips_to_exact_variant() {
        for (expected_route, path) in named_routes() {
            assert_eq!(Route::from_str(path).unwrap(), expected_route);
        }
    }

    #[test]
    fn precedence_ritual_over_habit_detail() {
        let route = Route::from_str("/habit/xyz/ritual").unwrap();
        assert_eq!(
            route,
            Route::Ritual {
                id: "xyz".to_string()
            }
        );
    }

    #[test]
    fn catch_all_parses_unknown_path_to_not_found() {
        let route = Route::from_str("/does/not/exist").unwrap();
        assert_eq!(
            route,
            Route::NotFound {
                segments: vec!["does".to_string(), "not".to_string(), "exist".to_string()]
            }
        );
    }

    mod system_back {
        use crate::composition::Services;
        use crate::i18n::use_locale_for_tests;
        use crate::route::Route;
        use dioxus::document::{Document, Eval, EvalError, Evaluator};
        use dioxus::history::{History, MemoryHistory, provide_history_context};
        use dioxus::prelude::*;
        use dioxus_core::NoOpMutations;
        use kayzen_core::habit_management::domain::goal::Goal;
        use kayzen_core::habit_management::domain::habit::Habit;
        use kayzen_core::habit_management::domain::habit_id::HabitId;
        use kayzen_core::habit_management::domain::habit_repository::HabitRepository;
        use kayzen_core::habit_management::domain::habit_title::HabitTitle;
        use kayzen_core::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
        use kayzen_core::shared::local_date::LocalDate;
        use serde_json::Value;
        use std::cell::RefCell;
        use std::collections::VecDeque;
        use std::rc::Rc;
        use std::task::{Context, Poll};

        #[derive(Default)]
        struct PageChannel {
            presses: RefCell<VecDeque<Value>>,
            told: RefCell<Vec<Value>>,
        }

        struct ChannelEvaluator(Rc<PageChannel>);

        impl Evaluator for ChannelEvaluator {
            fn poll_join(&mut self, _: &mut Context<'_>) -> Poll<Result<Value, EvalError>> {
                Poll::Pending
            }

            fn poll_recv(&mut self, _: &mut Context<'_>) -> Poll<Result<Value, EvalError>> {
                match self.0.presses.borrow_mut().pop_front() {
                    Some(press) => Poll::Ready(Ok(press)),
                    None => Poll::Pending,
                }
            }

            fn send(&self, data: Value) -> Result<(), EvalError> {
                self.0.told.borrow_mut().push(data);
                Ok(())
            }
        }

        struct WebViewStub {
            channel: Rc<PageChannel>,
            scripts: RefCell<Vec<String>>,
            owner: generational_box::Owner,
        }

        impl Document for WebViewStub {
            fn eval(&self, js: String) -> Eval {
                self.scripts.borrow_mut().push(js);
                Eval::new(
                    self.owner
                        .insert(Box::new(ChannelEvaluator(self.channel.clone()))),
                )
            }
        }

        #[derive(Props, Clone)]
        struct PageProps {
            web_view: Rc<WebViewStub>,
            history: Rc<MemoryHistory>,
        }

        impl PartialEq for PageProps {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.web_view, &other.web_view)
                    && Rc::ptr_eq(&self.history, &other.history)
            }
        }

        fn services_with_one_habit() -> Services {
            let repository: Rc<dyn HabitRepository> = Rc::new(InMemoryHabitRepository::new());
            repository.save(&Habit::new(
                HabitId::new("h-1").unwrap(),
                HabitTitle::new("Lire une page".to_string()).unwrap(),
                Goal::new(5).unwrap(),
                LocalDate::from_epoch_day(20_000),
            ));
            Services::with_repository(repository)
        }

        fn page(props: PageProps) -> Element {
            use_locale_for_tests();
            use_hook(|| provide_history_context(props.history.clone()));
            use_context_provider(|| props.web_view.clone() as Rc<dyn Document>);
            use_context_provider(services_with_one_habit);
            rsx! {
                Router::<Route> {}
            }
        }

        struct Pressed {
            route: String,
            told: Vec<Value>,
            scripts: Vec<String>,
        }

        fn open_and_press_back(path: &str, presses: usize) -> Pressed {
            let channel = Rc::new(PageChannel::default());
            for _ in 0..presses {
                channel.presses.borrow_mut().push_back(Value::Bool(true));
            }
            let web_view = Rc::new(WebViewStub {
                channel: channel.clone(),
                scripts: RefCell::new(Vec::new()),
                owner: generational_box::Owner::default(),
            });
            let history = Rc::new(MemoryHistory::with_initial_path(path));
            let mut vdom = VirtualDom::new_with_props(
                page,
                PageProps {
                    web_view: web_view.clone(),
                    history: history.clone(),
                },
            );
            vdom.rebuild_in_place();
            for _ in 0..8 {
                vdom.process_events();
                vdom.render_immediate(&mut NoOpMutations);
            }
            Pressed {
                route: history.current_route(),
                told: channel.told.take(),
                scripts: web_view.scripts.take(),
            }
        }

        // @scenario: system-back/S1
        #[test]
        fn system_back_s1_when_back_is_pressed_then_the_screen_one_level_up_is_shown() {
            for (screen, parent) in [
                ("/habit/h-1/ritual", "/habit/h-1"),
                ("/habit/h-1", "/"),
                ("/add", "/"),
                ("/week", "/"),
                ("/anchored", "/"),
                ("/paused", "/"),
                ("/does-not-exist", "/"),
            ] {
                let pressed = open_and_press_back(screen, 1);

                assert_eq!(pressed.route, parent, "Back on {screen} must show {parent}");
            }
        }

        // @scenario: system-back/S2
        #[test]
        fn system_back_s2_given_today_then_the_page_hands_back_to_the_system() {
            let on_today = open_and_press_back("/", 0);

            assert_eq!(
                on_today.told,
                vec![Value::Bool(false)],
                "on Today the page must tell the shell it has no level up, so Back leaves the app"
            );

            let back_to_today = open_and_press_back("/habit/h-1", 1);

            assert_eq!(back_to_today.route, "/");
            assert_eq!(
                back_to_today.told,
                vec![Value::Bool(true), Value::Bool(false)],
                "the page must retell the shell on each screen change: up on the detail, \
                 none once Back reached Today"
            );
            assert_eq!(
                back_to_today.scripts,
                vec![crate::back_key::BACK_JS.to_string()],
                "the back script is installed once and outlives the screen change"
            );
        }
    }
}
