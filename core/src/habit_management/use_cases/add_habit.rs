use crate::habit_management::domain::goal::Goal;
use crate::habit_management::domain::habit::{Habit, HabitError};
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::habit_management::domain::habit_title::HabitTitle;
use crate::habit_management::domain::lifecycle_state::LifecycleState;
use crate::shared::clock::Clock;
use crate::shared::guid_generator::GuidGenerator;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub enum AddHabitError {
    InvalidHabit(HabitError),
    DuplicateHabit,
    DailyLifeFull { max: usize },
}

impl fmt::Display for AddHabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddHabitError::InvalidHabit(error) => write!(f, "{error}"),
            AddHabitError::DuplicateHabit => {
                write!(f, "a habit with this title is already in your daily life")
            }
            AddHabitError::DailyLifeFull { max } => {
                write!(
                    f,
                    "your daily life already holds the maximum of {max} habits"
                )
            }
        }
    }
}

impl Error for AddHabitError {}

/// Command use case: adds a habit to the daily life in a single write. Formerly
/// two writes coordinated through a `HabitBoard` aggregate and a published
/// event (`RequestHabit` + `CreateHabitOnRequest`); the "at most
/// `Habit::MAX_IN_DAILY_LIFE`" rule is set-based validation over the existing
/// `HabitRepository`, not an aggregate invariant, so it belongs here.
#[derive(Clone)]
pub struct AddHabit {
    repository: Rc<dyn HabitRepository>,
    guid_generator: Rc<dyn GuidGenerator>,
    clock: Rc<dyn Clock>,
}

impl AddHabit {
    pub fn new(
        repository: Rc<dyn HabitRepository>,
        guid_generator: Rc<dyn GuidGenerator>,
        clock: Rc<dyn Clock>,
    ) -> AddHabit {
        AddHabit {
            repository,
            guid_generator,
            clock,
        }
    }

    pub fn execute(&self, title: String, goal: u32) -> Result<(), AddHabitError> {
        let id =
            HabitId::new(&self.guid_generator.generate()).map_err(AddHabitError::InvalidHabit)?;
        let title = HabitTitle::new(title).map_err(AddHabitError::InvalidHabit)?;
        let goal = Goal::new(goal).map_err(AddHabitError::InvalidHabit)?;

        let in_daily_life: Vec<Habit> = self
            .repository
            .all()
            .into_iter()
            .filter(|habit| habit.state() != LifecycleState::Anchored)
            .collect();

        if in_daily_life
            .iter()
            .any(|habit| habit.title().matches(&title))
        {
            return Err(AddHabitError::DuplicateHabit);
        }

        if in_daily_life.len() >= Habit::MAX_IN_DAILY_LIFE {
            return Err(AddHabitError::DailyLifeFull {
                max: Habit::MAX_IN_DAILY_LIFE,
            });
        }

        self.repository
            .save(&Habit::new(id, title, goal, self.clock.today()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::shared::clock::FixedClock;
    use crate::shared::local_date::LocalDate;
    use std::cell::Cell;

    const CREATED_ON: i64 = 20_000;

    struct StubGuidGenerator {
        guid: String,
    }

    impl GuidGenerator for StubGuidGenerator {
        fn generate(&self) -> String {
            self.guid.clone()
        }
    }

    /// Hands out a different guid on every call, so a use case that asked for
    /// one id and reused it would be caught.
    struct CountingGuidGenerator {
        calls: Cell<u32>,
    }

    impl GuidGenerator for CountingGuidGenerator {
        fn generate(&self) -> String {
            let next = self.calls.get() + 1;
            self.calls.set(next);
            format!("guid-{next}")
        }
    }

    /// Hands out an id longer than HabitId::MAX_LEN, so the use case's own
    /// refusal path (not the adapter's id-generation logic) is what gets
    /// exercised.
    struct OutOfBoundGuidGenerator;

    impl GuidGenerator for OutOfBoundGuidGenerator {
        fn generate(&self) -> String {
            "h".repeat(HabitId::MAX_LEN + 1)
        }
    }

    fn a_habit(id: &str, title: &str, goal: u32) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new(String::from(title)).unwrap(),
            Goal::new(goal).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn add_habit_with(guid: &str, repository: Rc<dyn HabitRepository>) -> AddHabit {
        AddHabit::new(
            repository,
            Rc::new(StubGuidGenerator {
                guid: guid.to_string(),
            }) as Rc<dyn GuidGenerator>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(CREATED_ON))) as Rc<dyn Clock>,
        )
    }

    fn a_fresh_add_habit() -> (AddHabit, Rc<InMemoryHabitRepository>) {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let add_habit = add_habit_with(
            "fixed-guid",
            Rc::clone(&repository) as Rc<dyn HabitRepository>,
        );
        (add_habit, repository)
    }

    fn a_daily_life_holding_five_habits() -> Rc<InMemoryHabitRepository> {
        let repository = Rc::new(InMemoryHabitRepository::new());

        for n in 1..=Habit::MAX_IN_DAILY_LIFE {
            let add_habit = add_habit_with(
                &format!("guid-{n}"),
                Rc::clone(&repository) as Rc<dyn HabitRepository>,
            );

            let result = add_habit.execute(format!("Habit number {n}"), 1);

            assert!(result.is_ok());
        }

        repository
    }

    // @scenario: add-habit/S1
    #[test]
    fn adding_a_habit_stores_it_with_its_generated_id_title_and_goal() {
        let cases = vec![
            (String::from("Read one page"), 5),
            ("a".repeat(HabitTitle::MIN_LEN), 5),
            ("a".repeat(HabitTitle::MAX_LEN), 5),
        ];

        for (title, goal) in cases {
            let (add_habit, repository) = a_fresh_add_habit();

            let result = add_habit.execute(title.clone(), goal);

            assert_eq!(result, Ok(()));
            assert_eq!(repository.all(), vec![a_habit("fixed-guid", &title, goal)]);
        }
    }

    #[test]
    fn every_added_habit_gets_its_own_freshly_generated_id() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let add_habit = AddHabit::new(
            Rc::clone(&repository) as Rc<dyn HabitRepository>,
            Rc::new(CountingGuidGenerator {
                calls: Cell::new(0),
            }) as Rc<dyn GuidGenerator>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(CREATED_ON))) as Rc<dyn Clock>,
        );

        let first = add_habit.execute(String::from("Read one page"), 5);
        let second = add_habit.execute(String::from("Move a little"), 5);

        assert_eq!(first, Ok(()));
        assert_eq!(second, Ok(()));
        let ids: Vec<String> = repository
            .all()
            .into_iter()
            .map(|habit| habit.id().value().to_string())
            .collect();
        assert_eq!(ids, vec!["guid-1".to_string(), "guid-2".to_string()]);
    }

    // @scenario: add-habit/S2
    #[test]
    fn a_goal_above_five_minutes_is_accepted() {
        let (add_habit, repository) = a_fresh_add_habit();

        let result = add_habit.execute(String::from("Run a marathon"), 6);

        assert!(result.is_ok());
        assert_eq!(repository.all().len(), 1);
    }

    #[test]
    fn display_formats_each_error_variant_with_the_expected_message() {
        let cases = vec![
            (
                AddHabitError::DuplicateHabit,
                "a habit with this title is already in your daily life".to_string(),
            ),
            (
                AddHabitError::DailyLifeFull { max: 5 },
                "your daily life already holds the maximum of 5 habits".to_string(),
            ),
            (
                AddHabitError::InvalidHabit(HabitError::TitleLength { min: 1, max: 50 }),
                HabitError::TitleLength { min: 1, max: 50 }.to_string(),
            ),
            (
                AddHabitError::InvalidHabit(HabitError::GoalTooSmall { min: 1 }),
                "a goal must be at least 1 minute(s) per day".to_string(),
            ),
            (
                AddHabitError::InvalidHabit(HabitError::IdLength { min: 1, max: 64 }),
                "an id size must be between 1 and 64 characters".to_string(),
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    // @scenario: add-habit/S3
    #[test]
    fn adding_an_invalid_habit_returns_an_error_and_stores_nothing() {
        let cases: Vec<(String, u32, AddHabitError)> = vec![
            (
                String::from("Tiny"),
                0,
                AddHabitError::InvalidHabit(HabitError::GoalTooSmall { min: 1 }),
            ),
            (
                String::new(),
                5,
                AddHabitError::InvalidHabit(HabitError::TitleLength {
                    min: HabitTitle::MIN_LEN,
                    max: HabitTitle::MAX_LEN,
                }),
            ),
            (
                "a".repeat(HabitTitle::MAX_LEN + 1),
                5,
                AddHabitError::InvalidHabit(HabitError::TitleLength {
                    min: HabitTitle::MIN_LEN,
                    max: HabitTitle::MAX_LEN,
                }),
            ),
        ];

        for (title, goal, expected_error) in cases {
            let (add_habit, repository) = a_fresh_add_habit();

            let result = add_habit.execute(title, goal);

            assert_eq!(result, Err(expected_error));
            assert!(repository.all().is_empty());
        }
    }

    // No Gherkin scenario names this path yet either (invalid-generated-id
    // refusal, T1 conformance with adr-0001) — flagged under "Open questions",
    // matching the same gap already noted in get_habit_detail.rs / mark_done.rs.
    #[test]
    fn a_generated_id_outside_the_bound_is_refused_and_stores_nothing() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let add_habit = AddHabit::new(
            Rc::clone(&repository) as Rc<dyn HabitRepository>,
            Rc::new(OutOfBoundGuidGenerator) as Rc<dyn GuidGenerator>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(CREATED_ON))) as Rc<dyn Clock>,
        );

        let result = add_habit.execute(String::from("Read one page"), 5);

        assert_eq!(
            result,
            Err(AddHabitError::InvalidHabit(HabitError::IdLength {
                min: HabitId::MIN_LEN,
                max: HabitId::MAX_LEN,
            }))
        );
        assert!(repository.all().is_empty());
    }

    // @scenario: add-habit/S4
    #[test]
    fn a_sixth_habit_is_rejected_on_a_full_daily_life_and_stores_nothing_new() {
        assert_eq!(Habit::MAX_IN_DAILY_LIFE, 5);

        let repository = a_daily_life_holding_five_habits();
        let sixth_add_habit =
            add_habit_with("guid-6", Rc::clone(&repository) as Rc<dyn HabitRepository>);

        let result = sixth_add_habit.execute(String::from("One habit too many"), 1);

        assert_eq!(
            result,
            Err(AddHabitError::DailyLifeFull {
                max: Habit::MAX_IN_DAILY_LIFE
            })
        );
        assert_eq!(repository.all().len(), Habit::MAX_IN_DAILY_LIFE);
    }

    // @scenario: add-habit/S5
    #[test]
    fn a_duplicate_title_is_rejected_ignoring_case_and_whitespace() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let first = add_habit_with("guid-1", Rc::clone(&repository) as Rc<dyn HabitRepository>);
        let second = add_habit_with("guid-2", Rc::clone(&repository) as Rc<dyn HabitRepository>);

        let first_result = first.execute(String::from("Lire une page"), 2);
        let second_result = second.execute(String::from("  lire une page  "), 2);

        assert!(first_result.is_ok());
        assert_eq!(second_result, Err(AddHabitError::DuplicateHabit));
        assert_eq!(repository.all().len(), 1);
    }

    // @scenario: add-habit/S6
    #[test]
    fn a_duplicate_title_on_a_full_daily_life_is_rejected_as_duplicate_not_full() {
        let repository = a_daily_life_holding_five_habits();
        let duplicate_add_habit =
            add_habit_with("guid-6", Rc::clone(&repository) as Rc<dyn HabitRepository>);

        let result = duplicate_add_habit.execute(String::from("Habit number 1"), 1);

        assert_eq!(result, Err(AddHabitError::DuplicateHabit));
        assert_eq!(repository.all().len(), Habit::MAX_IN_DAILY_LIFE);
    }

    // Test List — AddHabit threading the Clock (adr-0007 AD-1):
    // - the persisted habit's first step is dated with the clock's "today",
    //   not a default/hardcoded date.
    #[test]
    fn the_persisted_habit_is_created_on_the_clocks_today() {
        let (add_habit, repository) = a_fresh_add_habit();

        add_habit
            .execute(String::from("Read one page"), 2)
            .expect("valid habit");

        let habit = repository
            .get(&HabitId::new("fixed-guid").unwrap())
            .unwrap();
        let dates: Vec<LocalDate> = habit
            .step_history()
            .changes()
            .into_iter()
            .map(|step| step.on())
            .collect();
        assert_eq!(dates, vec![LocalDate::from_epoch_day(CREATED_ON)]);
    }
}
