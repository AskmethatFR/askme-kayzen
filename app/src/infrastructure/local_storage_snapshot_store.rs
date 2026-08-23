use kayzen_core::habit_management::infrastructure::snapshot_store::SnapshotStore;

/// `SnapshotStore` over the browser's `localStorage` — web's durable
/// substrate. Synchronous by construction: `Window::local_storage()` and
/// `Storage::get_item`/`set_item` are plain (non-`Promise`) `web-sys`
/// bindings, never `document::eval` (which is async and would reopen
/// adr-0013 condition (a)). One instance owns one fixed key; `Services::new()`
/// builds two of them (habits, quarantine), the same "not a key space"
/// contract `FileSnapshotStore` follows on desktop.
pub struct LocalStorageSnapshotStore {
    key: &'static str,
}

impl LocalStorageSnapshotStore {
    pub fn at(key: &'static str) -> LocalStorageSnapshotStore {
        LocalStorageSnapshotStore { key }
    }

    /// `None` covers every reason `localStorage` might be unreachable alike
    /// (no `Window`, storage disabled, a security-context refusal) — the
    /// caller cannot tell which and does not need to; a missing store reads
    /// as an empty board, the same fallback `load()` already gives an
    /// unreadable payload.
    fn storage(&self) -> Option<web_sys::Storage> {
        let window = web_sys::window()?;
        window.local_storage().ok()?
    }
}

impl SnapshotStore for LocalStorageSnapshotStore {
    fn load(&self) -> Option<String> {
        self.storage()?.get_item(self.key).ok()?
    }

    fn save(&self, payload: &str) {
        if let Some(storage) = self.storage() {
            let _ = storage.set_item(self.key, payload);
        }
    }
}
