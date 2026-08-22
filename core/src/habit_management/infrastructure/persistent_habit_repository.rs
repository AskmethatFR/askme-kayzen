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
}
