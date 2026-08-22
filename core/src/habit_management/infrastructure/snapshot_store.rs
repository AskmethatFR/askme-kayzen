/// Port to one durable location holding the whole habit snapshot — not a key
/// space, a single slot. Synchronous by hard constraint, not preference:
/// making either method `async` reopens condition (a) of adr-0013 (the
/// read-then-write window in `AddHabit` stops being atomic the moment a
/// suspension point exists between `all()` and `save()`).
pub trait SnapshotStore {
    fn load(&self) -> Option<String>;
    fn save(&self, payload: &str);
}

#[cfg(test)]
pub(crate) struct InMemorySnapshotStore {
    payload: std::cell::RefCell<Option<String>>,
}

#[cfg(test)]
impl InMemorySnapshotStore {
    pub(crate) fn empty() -> InMemorySnapshotStore {
        InMemorySnapshotStore {
            payload: std::cell::RefCell::new(None),
        }
    }

    pub(crate) fn seeded(payload: impl Into<String>) -> InMemorySnapshotStore {
        InMemorySnapshotStore {
            payload: std::cell::RefCell::new(Some(payload.into())),
        }
    }
}

#[cfg(test)]
impl SnapshotStore for InMemorySnapshotStore {
    fn load(&self) -> Option<String> {
        self.payload.borrow().clone()
    }

    fn save(&self, payload: &str) {
        *self.payload.borrow_mut() = Some(payload.to_string());
    }
}
