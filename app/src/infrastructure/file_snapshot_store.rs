use std::fs;
use std::io::{Read, Write};
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

    fn sibling_path(&self, suffix: &str) -> PathBuf {
        let mut sibling = self.path.clone().into_os_string();
        sibling.push(suffix);
        PathBuf::from(sibling)
    }

    fn quarantine_refused_file(&self) {
        let _ = fs::rename(&self.path, self.sibling_path(".refused"));
    }
}

impl SnapshotStore for FileSnapshotStore {
    fn load(&self) -> Option<String> {
        let file = fs::File::open(&self.path).ok()?;
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() || metadata.len() > Self::MAX_PAYLOAD_BYTES {
            self.quarantine_refused_file();
            return None;
        }
        let mut payload = String::new();
        file.take(Self::MAX_PAYLOAD_BYTES + 1)
            .read_to_string(&mut payload)
            .ok()?;
        if payload.len() as u64 > Self::MAX_PAYLOAD_BYTES {
            self.quarantine_refused_file();
            return None;
        }
        Some(payload)
    }

    fn save(&self, payload: &str) {
        if let Some(parent) = self.path.parent() {
            let _ = create_owner_only_dir(parent);
        }
        let temp_path = self.sibling_path(".tmp");
        if write_owner_only_file(&temp_path, payload).is_err() {
            let _ = fs::remove_file(&temp_path);
            return;
        }
        if fs::rename(&temp_path, &self.path).is_err() {
            let _ = fs::remove_file(&temp_path);
        }
    }
}

#[cfg(unix)]
fn create_owner_only_dir(dir: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
fn create_owner_only_dir(dir: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)
}

#[cfg(unix)]
fn write_owner_only_file(path: &std::path::Path, payload: &str) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(payload.as_bytes())?;
    file.sync_all()
}

#[cfg(not(unix))]
fn write_owner_only_file(path: &std::path::Path, payload: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(payload.as_bytes())?;
    file.sync_all()
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
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock must be after the epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("kayzen-file-snapshot-store-test-{nanos}-{unique}"))
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

    // Built as a sparse file via `set_len` rather than actually writing the
    // cap's worth of bytes: this test must stay cheap to run on every
    // `cargo test`, and what it pins is "refused before being read", which a
    // sparse file exercises identically to a dense one -- the metadata
    // reports the same length either way.
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

    // A character device reports `len() == 0` like a FIFO but, unlike a
    // FIFO, never blocks on open -- safe to exercise directly.
    #[cfg(unix)]
    #[test]
    fn load_refuses_a_character_device_even_though_its_reported_length_is_zero() {
        let store = FileSnapshotStore::at(PathBuf::from("/dev/null"));

        assert_eq!(store.load(), None);
    }

    #[test]
    fn load_accepts_a_payload_of_exactly_the_cap_size() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        let file = fs::File::create(&path).expect("temp file must be creatable");
        file.set_len(FileSnapshotStore::MAX_PAYLOAD_BYTES)
            .expect("sparse file must be extendable");

        let store = FileSnapshotStore::at(path);

        let loaded = store
            .load()
            .expect("a payload of exactly the cap size must be accepted");
        assert_eq!(loaded.len() as u64, FileSnapshotStore::MAX_PAYLOAD_BYTES);
    }

    #[test]
    fn load_refuses_a_directory_at_the_path() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path_that_is_a_directory = dir;

        let store = FileSnapshotStore::at(path_that_is_a_directory);

        assert_eq!(store.load(), None);
    }

    #[test]
    fn a_refused_oversized_file_is_preserved_at_a_refused_sibling_instead_of_left_exposed() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        let file = fs::File::create(&path).expect("temp file must be creatable");
        file.set_len(FileSnapshotStore::MAX_PAYLOAD_BYTES + 1)
            .expect("sparse file must be extendable");

        let store = FileSnapshotStore::at(path.clone());
        assert_eq!(store.load(), None);

        assert!(
            !path.exists(),
            "expected the refused file moved away, not left at the primary path"
        );
        let sibling = dir.join("habits.json.refused");
        assert_eq!(
            fs::metadata(&sibling).map(|m| m.len()).ok(),
            Some(FileSnapshotStore::MAX_PAYLOAD_BYTES + 1),
            "expected the whole refused payload preserved at the sibling path"
        );
    }

    #[test]
    fn a_failed_write_leaves_the_previous_snapshot_intact() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        fs::write(&path, "previous-payload").expect("temp file must be writable");
        let store = FileSnapshotStore::at(path.clone());
        fs::create_dir_all(store.sibling_path(".tmp")).expect("temp dir must be creatable");

        store.save("payload-that-must-not-land");

        assert_eq!(
            fs::read_to_string(&path).ok(),
            Some("previous-payload".to_string()),
            "expected a failed write to leave the previous snapshot untouched"
        );
    }

    #[cfg(unix)]
    #[test]
    fn save_creates_the_directory_and_file_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = unused_temp_path();
        let path = dir.join("habits.json");

        FileSnapshotStore::at(path.clone()).save("payload");

        let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        let file_mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o700, "expected the created directory owner-only");
        assert_eq!(file_mode, 0o600, "expected the created file owner-only");
    }

    #[test]
    fn a_successful_save_leaves_no_temp_file_behind() {
        let path = unused_temp_path().join("habits.json");
        let store = FileSnapshotStore::at(path);

        store.save("payload");

        assert!(!store.sibling_path(".tmp").exists());
    }

    #[test]
    fn load_over_an_absent_path_attempts_no_rename() {
        let dir = unused_temp_path();
        let path = dir.join("habits.json");

        let store = FileSnapshotStore::at(path);

        assert_eq!(store.load(), None);
        assert!(
            !dir.exists(),
            "expected no directory created while there was nothing to preserve"
        );
    }
}
