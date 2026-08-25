use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::habit_management::domain::lifecycle_state::LifecycleState;

#[derive(Debug, PartialEq)]
pub enum ReadmitHabitError {
    HabitNotFound,
    NotAnchored,
    DuplicateHabit,
    DailyLifeFull { max: usize },
}

impl ReadmitHabitError {
    pub const ALL: [ReadmitHabitError; 4] = [
        ReadmitHabitError::HabitNotFound,
        ReadmitHabitError::NotAnchored,
        ReadmitHabitError::DuplicateHabit,
        ReadmitHabitError::DailyLifeFull {
            max: Habit::MAX_IN_DAILY_LIFE,
        },
    ];
}

impl fmt::Display for ReadmitHabitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReadmitHabitError::HabitNotFound => write!(f, "no habit with this id exists"),
            ReadmitHabitError::NotAnchored => {
                write!(f, "only an anchored habit can be readmitted")
            }
            ReadmitHabitError::DuplicateHabit => {
                write!(f, "a habit with this title is already in your daily life")
            }
            ReadmitHabitError::DailyLifeFull { max } => {
                write!(
                    f,
                    "your daily life already holds the maximum of {max} habits"
                )
            }
        }
    }
}

impl Error for ReadmitHabitError {}

/// Command use case: puts an anchored habit back into the daily life. No
/// `Clock` (adr-0007 AD-3): nothing about this transition is dated.
#[derive(Clone)]
pub struct ReadmitHabit {
    repository: Rc<dyn HabitRepository>,
}

impl ReadmitHabit {
    pub fn new(repository: Rc<dyn HabitRepository>) -> ReadmitHabit {
        ReadmitHabit { repository }
    }

    pub fn execute(&self, habit_id: &str) -> Result<(), ReadmitHabitError> {
        let id = HabitId::new(habit_id).map_err(|_| ReadmitHabitError::HabitNotFound)?;
        let mut habit = self
            .repository
            .get(&id)
            .ok_or(ReadmitHabitError::HabitNotFound)?;

        let in_daily_life: Vec<Habit> = self
            .repository
            .all()
            .into_iter()
            .filter(|habit| habit.state() != LifecycleState::Anchored && habit.id() != &id)
            .collect();

        let target_title = habit.title().clone();
        if in_daily_life
            .iter()
            .any(|candidate| candidate.title().matches(&target_title))
        {
            return Err(ReadmitHabitError::DuplicateHabit);
        }

        if in_daily_life.len() >= Habit::MAX_IN_DAILY_LIFE {
            return Err(ReadmitHabitError::DailyLifeFull {
                max: Habit::MAX_IN_DAILY_LIFE,
            });
        }

        habit
            .readmit()
            .map_err(|_| ReadmitHabitError::NotAnchored)?;
        self.repository.save(&habit);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
    use crate::habit_management::use_cases::add_habit::{AddHabit, AddHabitError};
    use crate::shared::clock::{Clock, FixedClock};
    use crate::shared::guid_generator::GuidGenerator;
    use crate::shared::local_date::LocalDate;
    use std::cell::Cell;

    const CREATED_ON: i64 = 19_990;

    fn a_habit(id: &str, title: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new(title.to_string()).unwrap(),
            Goal::new(5).unwrap(),
            LocalDate::from_epoch_day(CREATED_ON),
        )
    }

    fn readmit_over(repository: Rc<InMemoryHabitRepository>) -> ReadmitHabit {
        ReadmitHabit::new(repository as Rc<dyn HabitRepository>)
    }

    /// Wraps an in-memory store and counts every `save` — the "exactly one
    /// write after both checks" contract (and its "no write on refusal" twin)
    /// is what a save-before-checks or save-twice mutant would violate.
    struct CountingRepository {
        inner: Rc<InMemoryHabitRepository>,
        save_calls: Cell<usize>,
    }

    impl HabitRepository for CountingRepository {
        fn save(&self, habit: &Habit) {
            self.save_calls.set(self.save_calls.get() + 1);
            self.inner.save(habit);
        }

        fn all(&self) -> Vec<Habit> {
            self.inner.all()
        }

        fn get(&self, id: &HabitId) -> Option<Habit> {
            self.inner.get(id)
        }
    }

    fn readmit_over_counting(repository: Rc<CountingRepository>) -> ReadmitHabit {
        ReadmitHabit::new(repository as Rc<dyn HabitRepository>)
    }

    struct StubGuidGenerator {
        guid: String,
    }

    impl GuidGenerator for StubGuidGenerator {
        fn generate(&self) -> String {
            self.guid.clone()
        }
    }

    fn an_add_habit(guid: &str, repository: &Rc<InMemoryHabitRepository>) -> AddHabit {
        AddHabit::new(
            Rc::clone(repository) as Rc<dyn HabitRepository>,
            Rc::new(StubGuidGenerator {
                guid: guid.to_string(),
            }) as Rc<dyn GuidGenerator>,
            Rc::new(FixedClock::new(LocalDate::from_epoch_day(CREATED_ON))) as Rc<dyn Clock>,
        )
    }

    /// Adds one non-anchored habit per title through `AddHabit`, so every
    /// caller starts from a store holding exactly those habits in the daily
    /// life — the S2/S3 fixtures both need before they can readmit anything.
    fn a_daily_life_seeded_with(titles: &[&str]) -> Rc<InMemoryHabitRepository> {
        let repository = Rc::new(InMemoryHabitRepository::new());

        for (n, title) in titles.iter().enumerate() {
            let guid = format!("guid-{}", n + 1);
            an_add_habit(&guid, &repository)
                .execute(title.to_string(), 1)
                .expect("valid habit");
        }

        repository
    }

    fn an_anchored_habit_in(repository: &Rc<InMemoryHabitRepository>, id: &str, title: &str) {
        let mut habit = a_habit(id, title);
        habit.anchor().expect("a fresh habit is active");
        repository.save(&habit);
    }

    #[test]
    fn display_formats_the_error_with_the_expected_message() {
        assert_eq!(
            ReadmitHabitError::HabitNotFound.to_string(),
            "no habit with this id exists"
        );
        assert_eq!(
            ReadmitHabitError::NotAnchored.to_string(),
            "only an anchored habit can be readmitted"
        );
        assert_eq!(
            ReadmitHabitError::DuplicateHabit.to_string(),
            "a habit with this title is already in your daily life"
        );
        assert_eq!(
            ReadmitHabitError::DailyLifeFull { max: 5 }.to_string(),
            "your daily life already holds the maximum of 5 habits"
        );
    }

    // @scenario: readmit-habit/S1
    #[test]
    fn readmitting_an_anchored_habit_makes_it_active_and_leaves_its_histories_untouched() {
        let inner = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", "Read one page");
        let grown_on = LocalDate::from_epoch_day(CREATED_ON + 1);
        habit.grow(grown_on);
        habit.toggle_done(grown_on);
        habit.anchor().expect("a fresh habit is active");
        let step_count = habit.step_history().changes().len();
        inner.save(&habit);
        let counting = Rc::new(CountingRepository {
            inner,
            save_calls: Cell::new(0),
        });
        let readmit_habit = readmit_over_counting(Rc::clone(&counting));

        let result = readmit_habit.execute("h-1");

        assert_eq!(result, Ok(()));
        let readmitted = counting.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(readmitted.state(), LifecycleState::Active);
        assert_eq!(readmitted.step_history().changes().len(), step_count);
        assert!(
            readmitted.is_done_on(grown_on),
            "readmitting must leave the completion history untouched"
        );
        assert_eq!(
            counting.save_calls.get(),
            1,
            "a successful readmit must write the habit exactly once"
        );
    }

    #[test]
    fn readmitting_an_active_habit_is_refused() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", "Read one page"));
        let readmit_habit = readmit_over(Rc::clone(&repository));

        let result = readmit_habit.execute("h-1");

        assert_eq!(result, Err(ReadmitHabitError::NotAnchored));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.state(), LifecycleState::Active);
    }

    #[test]
    fn readmitting_a_paused_habit_is_refused() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let mut habit = a_habit("h-1", "Read one page");
        habit.pause().expect("a fresh habit is active");
        repository.save(&habit);
        let readmit_habit = readmit_over(Rc::clone(&repository));

        let result = readmit_habit.execute("h-1");

        assert_eq!(result, Err(ReadmitHabitError::NotAnchored));
        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(habit.state(), LifecycleState::Paused);
    }

    #[test]
    fn readmitting_an_unknown_habit_is_rejected() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        let readmit_habit = readmit_over(repository);

        let result = readmit_habit.execute("missing");

        assert_eq!(result, Err(ReadmitHabitError::HabitNotFound));
    }

    #[test]
    fn readmitting_an_id_outside_the_bound_is_refused_without_panicking() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        repository.save(&a_habit("h-1", "Read one page"));
        let readmit_habit = readmit_over(repository);
        let too_long = "h".repeat(HabitId::MAX_LEN + 1);

        let result = readmit_habit.execute(&too_long);

        assert_eq!(result, Err(ReadmitHabitError::HabitNotFound));
    }

    // @scenario: readmit-habit/S2
    #[test]
    fn readmission_is_refused_when_the_daily_life_is_full_and_the_habit_stays_anchored() {
        let repository = a_daily_life_seeded_with(&[
            "Habit number 1",
            "Habit number 2",
            "Habit number 3",
            "Habit number 4",
            "Habit number 5",
        ]);
        an_anchored_habit_in(&repository, "h-anchored", "Read one page");
        let counting = Rc::new(CountingRepository {
            inner: Rc::clone(&repository),
            save_calls: Cell::new(0),
        });
        let readmit_habit = readmit_over_counting(Rc::clone(&counting));

        let result = readmit_habit.execute("h-anchored");

        assert_eq!(
            result,
            Err(ReadmitHabitError::DailyLifeFull {
                max: Habit::MAX_IN_DAILY_LIFE
            })
        );
        let habit = counting.get(&HabitId::new("h-anchored").unwrap()).unwrap();
        assert_eq!(habit.state(), LifecycleState::Anchored);
        assert_eq!(counting.all().len(), Habit::MAX_IN_DAILY_LIFE + 1);
        assert_eq!(counting.save_calls.get(), 0);
    }

    // @scenario: readmit-habit/S3
    #[test]
    fn readmission_is_refused_when_the_title_is_already_back_in_the_daily_life() {
        let repository = a_daily_life_seeded_with(&["lire une page"]);
        an_anchored_habit_in(&repository, "h-anchored", "Lire une page");
        let counting = Rc::new(CountingRepository {
            inner: Rc::clone(&repository),
            save_calls: Cell::new(0),
        });
        let readmit_habit = readmit_over_counting(Rc::clone(&counting));

        let result = readmit_habit.execute("h-anchored");

        assert_eq!(result, Err(ReadmitHabitError::DuplicateHabit));
        let habit = counting.get(&HabitId::new("h-anchored").unwrap()).unwrap();
        assert_eq!(habit.state(), LifecycleState::Anchored);
        assert_eq!(counting.save_calls.get(), 0);
    }

    #[test]
    fn a_duplicate_title_on_a_full_daily_life_is_rejected_as_duplicate_not_full() {
        let repository = a_daily_life_seeded_with(&[
            "Lire une page",
            "Habit number 2",
            "Habit number 3",
            "Habit number 4",
            "Habit number 5",
        ]);
        an_anchored_habit_in(&repository, "h-anchored", "lire une page");
        let readmit_habit = readmit_over(Rc::clone(&repository));

        let result = readmit_habit.execute("h-anchored");

        assert_eq!(result, Err(ReadmitHabitError::DuplicateHabit));
    }

    #[test]
    fn a_paused_habit_counts_toward_the_daily_life_cap() {
        let repository = a_daily_life_seeded_with(&[
            "Habit number 1",
            "Habit number 2",
            "Habit number 3",
            "Habit number 4",
        ]);
        let mut paused = a_habit("h-paused", "Paused habit");
        paused.pause().expect("a fresh habit is active");
        repository.save(&paused);
        an_anchored_habit_in(&repository, "h-anchored", "Read one page");
        let readmit_habit = readmit_over(Rc::clone(&repository));

        let result = readmit_habit.execute("h-anchored");

        assert_eq!(
            result,
            Err(ReadmitHabitError::DailyLifeFull {
                max: Habit::MAX_IN_DAILY_LIFE
            })
        );
    }

    #[test]
    fn an_anchored_peer_with_the_same_title_does_not_make_a_duplicate() {
        let repository = Rc::new(InMemoryHabitRepository::new());
        an_anchored_habit_in(&repository, "h-a", "Lire une page");
        an_anchored_habit_in(&repository, "h-b", "Lire une page");
        let readmit_habit = readmit_over(Rc::clone(&repository));

        let result = readmit_habit.execute("h-a");

        assert_eq!(result, Ok(()));
        let readmitted = repository.get(&HabitId::new("h-a").unwrap()).unwrap();
        assert_eq!(readmitted.state(), LifecycleState::Active);
        let peer = repository.get(&HabitId::new("h-b").unwrap()).unwrap();
        assert_eq!(peer.state(), LifecycleState::Anchored);
    }

    #[test]
    fn after_a_successful_readmit_a_sixth_add_habit_is_refused_again() {
        let repository = a_daily_life_seeded_with(&[
            "Habit number 1",
            "Habit number 2",
            "Habit number 3",
            "Habit number 4",
        ]);
        an_anchored_habit_in(&repository, "h-anchored", "Read one page");
        let readmit_habit = readmit_over(Rc::clone(&repository));
        readmit_habit
            .execute("h-anchored")
            .expect("the daily life holds four non-anchored habits");

        let result =
            an_add_habit("guid-6", &repository).execute(String::from("One habit too many"), 1);

        assert_eq!(
            result,
            Err(AddHabitError::DailyLifeFull {
                max: Habit::MAX_IN_DAILY_LIFE
            }),
            "expected the readmitted habit's seat to be taken again, got: {result:?}"
        );
    }

    #[test]
    fn after_a_successful_readmit_the_same_title_is_refused_as_duplicate_again() {
        let repository = a_daily_life_seeded_with(&["Habit number 1"]);
        an_anchored_habit_in(&repository, "h-anchored", "Read one page");
        let readmit_habit = readmit_over(Rc::clone(&repository));
        readmit_habit
            .execute("h-anchored")
            .expect("the daily life holds one non-anchored habit");

        let result = an_add_habit("guid-2", &repository).execute(String::from("read one page"), 1);

        assert_eq!(result, Err(AddHabitError::DuplicateHabit));
    }
}
