use std::fs;
use std::path::PathBuf;

use kayzen_core::habit_management::infrastructure::snapshot_store::SnapshotStore;

/// `SnapshotStore` over a single file on disk — desktop's durable substrate
/// (web stays on `LocalStorageSnapshotStore`, wired in the sibling module).
/// One instance owns one fixed path; `Services::new()` builds two of them
/// (habits, quarantine) rather than one store keyed by name, keeping the
/// "not a key space" contract from `SnapshotStore`'s own doc comment.
pub struct FileSnapshotStore {
    path: PathBuf,
}

impl FileSnapshotStore {
    /// Mirrors `HabitSnapshotCodec::MAX_PAYLOAD_BYTES` in `kayzen-core`
    /// (currently 4 MiB). That constant is `pub(crate)` to the core crate's
    /// `habit_snapshot_codec` module — deliberately: it is not part of the
    /// codec's public surface, and adr-0010's single-door discipline is
    /// exactly why `kayzen-app` cannot reach across the crate boundary for
    /// it. A file adapter still needs its own bound, and earlier than the
    /// codec's: `load()` returns a `String` with no way to say "too big", so
    /// a size cap applied only at parse time is already too late — the
    /// adapter has materialised the whole payload in memory before the codec
    /// ever sees it. The two numbers are restated, not shared; keep them in
    /// sync by hand if the codec's bound ever changes.
    pub(crate) const MAX_PAYLOAD_BYTES: u64 = 4 * 1024 * 1024;

    pub fn at(path: PathBuf) -> FileSnapshotStore {
        FileSnapshotStore { path }
    }
}

impl SnapshotStore for FileSnapshotStore {
    fn load(&self) -> Option<String> {
        let metadata = fs::metadata(&self.path).ok()?;
        if metadata.len() > Self::MAX_PAYLOAD_BYTES {
            return None;
        }
        fs::read_to_string(&self.path).ok()
    }

    fn save(&self, payload: &str) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&self.path, payload);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A fresh, never-created path under the OS temp directory — every test
    /// gets its own, so none can observe another's file or directory.
    /// Deliberately never touches `dirs::data_dir()`: that is the real
    /// user's machine, and this suite must never write there.
    fn unused_temp_path() -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "kayzen-file-snapshot-store-test-{}-{}",
            std::process::id(),
            unique
        ))
    }

    #[test]
    fn a_saved_payload_is_read_back_by_a_second_instance_over_the_same_path() {
        let path = unused_temp_path().join("habits.json");
        FileSnapshotStore::at(path.clone()).save(r#"{"v":1,"habits":[]}"#);

        let reopened = FileSnapshotStore::at(path);

        assert_eq!(reopened.load(), Some(r#"{"v":1,"habits":[]}"#.to_string()));
    }

    #[test]
    fn load_returns_none_when_the_directory_was_never_created() {
        let path = unused_temp_path().join("habits.json");

        let store = FileSnapshotStore::at(path);

        assert_eq!(store.load(), None);
    }

    #[test]
    fn save_creates_the_parent_directory_when_it_does_not_exist() {
        let dir = unused_temp_path();
        let path = dir.join("habits.json");
        assert!(!dir.exists(), "the temp dir must start out absent");

        FileSnapshotStore::at(path).save("payload");

        assert!(dir.is_dir());
    }

    // Security F-4 (retry-2, the same finding S1's codec already answers for
    // the parsed payload): `SnapshotStore::load() -> Option<String>` gives no
    // way to say "too big", and the codec's cap only bounds the parse -- by
    // then a file adapter has already materialised the whole payload in
    // memory. A `HashMap`/Vec-backed store never faces this (its size is
    // memory itself); a *file* can hold arbitrarily more than RAM. Built as a
    // sparse file via `set_len` rather than actually writing the cap's worth
    // of bytes: this test must stay cheap to run on every `cargo test`, and
    // what it pins is "refused before being read", which a sparse file
    // exercises identically to a dense one -- `fs::metadata` reports the same
    // length either way.
    #[test]
    fn load_refuses_a_payload_larger_than_the_cap_without_reading_it_into_memory() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        let file = fs::File::create(&path).expect("temp file must be creatable");
        file.set_len(FileSnapshotStore::MAX_PAYLOAD_BYTES + 1)
            .expect("sparse file must be extendable");

        let store = FileSnapshotStore::at(path);

        assert_eq!(store.load(), None);
    }
}
