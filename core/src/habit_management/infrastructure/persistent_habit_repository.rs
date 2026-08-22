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
    /// loss, never a silent one. Named `hydrated_from`, not `new`: this is
    /// the branchiest code in the slice (load/decode/quarantine), and
    /// `Habit::rehydrate` already established the reason a constructor here
    /// should never be literally called `new` — cargo-mutants hard-skips it
    /// (adr-0009).
    pub fn hydrated_from(
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
        PersistentHabitRepository::hydrated_from(store, Rc::new(InMemorySnapshotStore::empty()))
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
    fn the_aggregate_round_trips_completely_for_every_lifecycle_state() {
        // Same behaviour (a full round trip), divergent data (the state the
        // habit is in when saved) — one parameterized cycle rather than
        // three, per the collapse rule. Anchored is not a throwaway extra
        // row: before this test it was the only LifecycleState variant never
        // exercised through PersistentHabitRepository, and an Anchored/Active
        // mix-up on either side of the codec is exactly the kind of bug that
        // would slip through with Active and Paused alone.
        type Transition = (&'static str, fn(&mut Habit));
        let transitions: Vec<Transition> = vec![
            ("Active", |_habit: &mut Habit| {}),
            ("Paused", |habit: &mut Habit| {
                habit.pause().expect("a fresh habit is active");
            }),
            ("Anchored", |habit: &mut Habit| {
                habit.anchor().expect("a fresh habit is active");
            }),
        ];

        for (label, transition) in transitions {
            let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());
            let mut habit = a_habit("h-1");
            habit.grow(LocalDate::from_epoch_day(20_003));
            habit.lighten(LocalDate::from_epoch_day(20_010));
            habit.toggle_done(LocalDate::from_epoch_day(20_000));
            habit.toggle_done(LocalDate::from_epoch_day(20_011));
            transition(&mut habit);
            repository_over(Rc::clone(&store)).save(&habit);

            let reopened = repository_over(Rc::clone(&store));

            assert_eq!(
                reopened.get(&HabitId::new("h-1").unwrap()),
                Some(habit),
                "state {label} did not round-trip"
            );
        }
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

        let repository =
            PersistentHabitRepository::hydrated_from(Rc::clone(&store), Rc::clone(&quarantine));

        assert_eq!(repository.all(), Vec::new());
        assert_eq!(quarantine.load(), Some("not-json-at-all".to_string()));
    }

    #[test]
    fn a_payload_with_an_unknown_version_takes_the_unreadable_path_without_being_parsed() {
        let payload = r#"{"v":2,"habits":[{"id":"h-1","title":"Read one page","state":"Active","steps":[{"on":20000,"goal":3}],"completions":[]}]}"#;
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::seeded(payload));
        let quarantine: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());

        let repository =
            PersistentHabitRepository::hydrated_from(Rc::clone(&store), Rc::clone(&quarantine));

        assert_eq!(
            repository.all(),
            Vec::new(),
            "a well-formed habit under an unknown version must not be parsed into a domain object"
        );
        assert_eq!(quarantine.load(), Some(payload.to_string()));
    }

    // B9: the previous version of this test asserted on the same instance,
    // never reopening over the store — it duplicated
    // in_memory_habit_repository.rs's own overwrite test and stayed green
    // even with self.persist() deleted from save(). Reopening over the
    // store is what actually exercises persistence.
    #[test]
    fn saving_an_existing_id_overwrites_instead_of_duplicating_across_a_restart() {
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());
        let mut habit = a_habit("h-1");
        repository_over(Rc::clone(&store)).save(&habit);

        habit.toggle_done(LocalDate::from_epoch_day(20_000));
        repository_over(Rc::clone(&store)).save(&habit);

        let reopened = repository_over(Rc::clone(&store));
        assert_eq!(reopened.all(), vec![habit]);
    }

    // B1 / Security F-1: an out-of-range stored date used to reach
    // `Habit::minutes_practised` unbounded and unvalidated, turning a day-by-day
    // walk from `today` down to `created_on` into an attacker-controlled loop
    // (measured: ~2124 years extrapolated at i64::MIN, freezing on every launch).
    // `LocalDate::parse_stored` rejects it at the parse point, so it falls onto
    // the same all-or-nothing path as any other unreadable field.
    #[test]
    fn a_stored_date_outside_the_accepted_range_yields_zero_habits_and_is_quarantined() {
        let payload = format!(
            r#"{{"v":1,"habits":[{{"id":"h-1","title":"Read one page","state":"Active","steps":[{{"on":{},"goal":3}}],"completions":[]}}]}}"#,
            i64::MIN
        );
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::seeded(payload.clone()));
        let quarantine: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());

        let repository =
            PersistentHabitRepository::hydrated_from(Rc::clone(&store), Rc::clone(&quarantine));

        assert_eq!(repository.all(), Vec::new());
        assert_eq!(quarantine.load(), Some(payload));
    }

    // B6 / Security F-4: `load()` gave an S2 file adapter no way to say "too
    // big" — an unbounded fs::read_to_string is an OOM primitive at launch.
    // Bounded at the codec's parse point, same all-or-nothing path as any
    // other unreadable payload. Deliberately built as OTHERWISE-VALID JSON
    // (a huge but well-formed completions array, not garbage bytes): a
    // malformed-but-oversized payload would pass this test even with the
    // size check deleted, because `serde_json::from_str` would already
    // reject it for being unparsable — this shape isolates the size cap as
    // the acting mechanism.
    #[test]
    fn a_payload_larger_than_the_size_cap_yields_zero_habits_and_is_quarantined() {
        let mut completions = String::new();
        let mut day = 20_000i64;
        while completions.len() <= HabitSnapshotCodec::MAX_PAYLOAD_BYTES {
            if !completions.is_empty() {
                completions.push(',');
            }
            completions.push_str(&day.to_string());
            day += 1;
        }
        let payload = format!(
            r#"{{"v":1,"habits":[{{"id":"h-1","title":"Read one page","state":"Active","steps":[{{"on":20000,"goal":3}}],"completions":[{completions}]}}]}}"#
        );
        assert!(
            payload.len() > HabitSnapshotCodec::MAX_PAYLOAD_BYTES,
            "test payload must actually exceed the cap for this test to mean anything"
        );
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::seeded(payload.clone()));
        let quarantine: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());

        let repository =
            PersistentHabitRepository::hydrated_from(Rc::clone(&store), Rc::clone(&quarantine));

        assert_eq!(repository.all(), Vec::new());
        assert_eq!(quarantine.load(), Some(payload));
    }

    // B7 / Security F-6: `StepHistory::goal_on` scans `rest` from the end
    // looking for the last step dated on-or-before the queried day, which is
    // only correct if `rest` is chronologically ordered. `rehydrate` already
    // normalises a repeated goal (see the test above) but never re-sorted by
    // date — a stored payload with an out-of-order `on` would silently make
    // `goal_on` answer wrong for no visible error. Rejected here rather than
    // sorted: sorting would rewrite what was stored into an order it never
    // held, the same all-or-nothing stance already taken for every other
    // structurally-broken field (B1's out-of-range date, the unknown-version
    // case) rather than a quiet best-effort repair.
    #[test]
    fn a_stored_step_history_with_out_of_order_dates_yields_zero_habits_and_is_quarantined() {
        let payload = r#"{"v":1,"habits":[{"id":"h-1","title":"Read one page","state":"Active","steps":[{"on":20010,"goal":3},{"on":20000,"goal":5}],"completions":[]}]}"#;
        let store: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::seeded(payload));
        let quarantine: Rc<dyn SnapshotStore> = Rc::new(InMemorySnapshotStore::empty());

        let repository =
            PersistentHabitRepository::hydrated_from(Rc::clone(&store), Rc::clone(&quarantine));

        assert_eq!(repository.all(), Vec::new());
        assert_eq!(quarantine.load(), Some(payload.to_string()));
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
