use std::rc::Rc;

use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_repository::HabitRepository;
use crate::habit_management::infrastructure::habit_snapshot_codec::HabitSnapshotCodec;
use crate::habit_management::infrastructure::in_memory_habit_repository::InMemoryHabitRepository;
use crate::habit_management::infrastructure::snapshot_store::SnapshotStore;

/// Decorates `InMemoryHabitRepository` with durability: hydrates once at
/// construction from `store`, serves every read from memory, and rewrites the
/// whole snapshot on each write. The board caps at
/// `Habit::MAX_IN_DAILY_LIFE`, so a full rewrite costs a few kilobytes —
/// differential writes would only add complexity for no measurable gain.
/// `HabitRepository` itself is untouched: this is one more adapter behind the
/// same three methods, not a change to the port.
pub struct PersistentHabitRepository {
    store: Rc<dyn SnapshotStore>,
    inner: InMemoryHabitRepository,
}

impl PersistentHabitRepository {
    /// `quarantine` receives a byte-for-byte copy of `store`'s payload the
    /// one time it turns out to be unreadable, before that payload is ever
    /// overwritten by the first `save()` — a format bug costs a recoverable
    /// loss, never a silent one.
    pub fn new(
        store: Rc<dyn SnapshotStore>,
        quarantine: Rc<dyn SnapshotStore>,
    ) -> PersistentHabitRepository {
        let inner = InMemoryHabitRepository::new();

        if let Some(payload) = store.load() {
            match HabitSnapshotCodec::decode(&payload) {
                Some(habits) => {
                    for habit in habits {
                        inner.save(&habit);
                    }
                }
                None => quarantine.save(&payload),
            }
        }

        PersistentHabitRepository { store, inner }
    }

    fn persist(&self) {
        let payload = HabitSnapshotCodec::encode(&self.inner.all());
        self.store.save(&payload);
    }
}

impl HabitRepository for PersistentHabitRepository {
    fn save(&self, habit: &Habit) {
        self.inner.save(habit);
        self.persist();
    }

    fn all(&self) -> Vec<Habit> {
        self.inner.all()
    }

    fn get(&self, id: &HabitId) -> Option<Habit> {
        self.inner.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habit_management::domain::goal::Goal;
    use crate::habit_management::domain::habit_title::HabitTitle;
    use crate::habit_management::infrastructure::snapshot_store::InMemorySnapshotStore;
    use crate::shared::local_date::LocalDate;

    fn a_habit(id: &str) -> Habit {
        Habit::new(
            HabitId::new(id).unwrap(),
            HabitTitle::new("Read one page".to_string()).unwrap(),
            Goal::new(3).unwrap(),
            LocalDate::from_epoch_day(20_000),
        )
    }

    fn repository_over(store: Rc<dyn SnapshotStore>) -> PersistentHabitRepository {
        PersistentHabitRepository::new(store, Rc::new(InMemorySnapshotStore::empty()))
    }

    #[test]
    fn a_saved_habit_is_returned_by_all_of_a_fresh_repository_over_the_same_store() {
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());
        let habit = a_habit("h-1");
        repository_over(Rc::clone(&store)).save(&habit);

        let reopened = repository_over(Rc::clone(&store));

        assert_eq!(reopened.all(), vec![habit]);
    }

    #[test]
    fn a_completion_survives_a_restart_at_the_exact_date() {
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());
        let mut habit = a_habit("h-1");
        habit.toggle_done(LocalDate::from_epoch_day(20_003));
        repository_over(Rc::clone(&store)).save(&habit);

        let reopened = repository_over(Rc::clone(&store));

        let rehydrated = reopened.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert!(rehydrated.is_done_on(LocalDate::from_epoch_day(20_003)));
        assert!(!rehydrated.is_done_on(LocalDate::from_epoch_day(20_004)));
    }

    #[test]
    fn the_aggregate_round_trips_completely() {
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());
        let mut habit = a_habit("h-1");
        habit.grow(LocalDate::from_epoch_day(20_003));
        habit.lighten(LocalDate::from_epoch_day(20_010));
        habit.toggle_done(LocalDate::from_epoch_day(20_000));
        habit.toggle_done(LocalDate::from_epoch_day(20_011));
        habit.pause().expect("a fresh habit is active");
        repository_over(Rc::clone(&store)).save(&habit);

        let reopened = repository_over(Rc::clone(&store));

        assert_eq!(reopened.get(&HabitId::new("h-1").unwrap()), Some(habit));
    }

    #[test]
    fn an_empty_store_yields_zero_habits() {
        let repository = repository_over(Rc::new(InMemorySnapshotStore::empty()));

        assert_eq!(repository.all(), Vec::new());
    }

    #[test]
    fn an_unreadable_payload_yields_zero_habits_and_is_found_in_the_quarantine_store() {
        let store: Rc<dyn SnapshotStore> =
            Rc::new(InMemorySnapshotStore::seeded("not-json-at-all"));
        let quarantine: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());

        let repository = PersistentHabitRepository::new(Rc::clone(&store), Rc::clone(&quarantine));

        assert_eq!(repository.all(), Vec::new());
        assert_eq!(quarantine.load(), Some("not-json-at-all".to_string()));
    }

    #[test]
    fn a_payload_with_an_unknown_version_takes_the_unreadable_path_without_being_parsed() {
        let payload = r#"{"v":2,"habits":[{"id":"h-1","title":"Read one page","state":"Active","steps":[{"on":20000,"goal":3}],"completions":[]}]}"#;
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::seeded(payload));
        let quarantine: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());

        let repository = PersistentHabitRepository::new(Rc::clone(&store), Rc::clone(&quarantine));

        assert_eq!(
            repository.all(),
            Vec::new(),
            "a well-formed habit under an unknown version must not be parsed into a domain object"
        );
        assert_eq!(quarantine.load(), Some(payload.to_string()));
    }

    #[test]
    fn saving_an_existing_id_overwrites_instead_of_duplicating() {
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());
        let repository = repository_over(Rc::clone(&store));
        let mut habit = a_habit("h-1");
        repository.save(&habit);

        habit.toggle_done(LocalDate::from_epoch_day(20_000));
        repository.save(&habit);

        assert_eq!(repository.all(), vec![habit]);
    }

    #[test]
    fn a_stored_step_history_with_two_consecutive_steps_at_the_same_goal_is_normalised_on_read() {
        let payload = r#"{"v":1,"habits":[{"id":"h-1","title":"Read one page","state":"Active","steps":[{"on":20000,"goal":3},{"on":20003,"goal":3}],"completions":[]}]}"#;
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::seeded(payload));

        let repository = repository_over(Rc::clone(&store));

        let habit = repository.get(&HabitId::new("h-1").unwrap()).unwrap();
        assert_eq!(
            habit.step_history().changes().len(),
            1,
            "the second step repeats the same goal and must collapse into the first"
        );
    }
}
