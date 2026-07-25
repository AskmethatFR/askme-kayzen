use crate::views::*;
use dioxus::prelude::*;

#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    Today {},
    #[route("/habit/:id/ritual")]
    Ritual { id: String },
    #[route("/habit/:id")]
    HabitDetail { id: String },
    #[route("/week")]
    Week {},
    #[route("/anchored")]
    Anchored {},
    #[route("/add")]
    AddHabit {},
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
}
