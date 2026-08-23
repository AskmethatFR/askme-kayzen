use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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

    const MAX_TEMP_FILE_ATTEMPTS: u8 = 5;

    pub fn at(path: PathBuf) -> FileSnapshotStore {
        FileSnapshotStore { path }
    }

    fn sibling_path(&self, suffix: &str) -> PathBuf {
        let mut sibling = self.path.clone().into_os_string();
        sibling.push(suffix);
        PathBuf::from(sibling)
    }

    /// A fresh name on every call, never reused within this process — the
    /// substrate for `create_new`'s all-or-nothing guarantee in `save()`: a
    /// plantable, predictable `.tmp` name is exactly what let an attacker
    /// pre-occupy it (mode laundering, symlink following, a permanently
    /// broken save). Uniqueness closes that off by construction instead of
    /// by inspecting what is already there.
    fn unique_temp_path(&self) -> PathBuf {
        static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let unique = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.sibling_path(&format!(".{nanos}-{unique}.tmp"))
    }

    fn write_to_unique_temp_file(&self, payload: &str) -> Option<PathBuf> {
        for _ in 0..Self::MAX_TEMP_FILE_ATTEMPTS {
            let temp_path = self.unique_temp_path();
            match write_owner_only_new_file(&temp_path, payload) {
                Ok(()) => return Some(temp_path),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => {
                    let _ = fs::remove_file(&temp_path);
                    return None;
                }
            }
        }
        None
    }

    fn refuse(&self) -> Option<String> {
        quarantine_refused_file(&self.path, &self.sibling_path(".refused"));
        None
    }
}

impl SnapshotStore for FileSnapshotStore {
    fn load(&self) -> Option<String> {
        let file = match open_for_load(&self.path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
            Err(_) => return self.refuse(),
        };
        let metadata = match file.metadata() {
            Ok(metadata) => metadata,
            Err(_) => return self.refuse(),
        };
        if !metadata.is_file() || metadata.len() > Self::MAX_PAYLOAD_BYTES {
            return self.refuse();
        }
        let mut payload = String::new();
        if file
            .take(Self::MAX_PAYLOAD_BYTES + 1)
            .read_to_string(&mut payload)
            .is_err()
        {
            return self.refuse();
        }
        if payload.len() as u64 > Self::MAX_PAYLOAD_BYTES {
            return self.refuse();
        }
        Some(payload)
    }

    fn save(&self, payload: &str) {
        if let Some(parent) = self.path.parent() {
            let _ = create_owner_only_dir(parent);
        }
        let Some(temp_path) = self.write_to_unique_temp_file(payload) else {
            return;
        };
        if fs::rename(&temp_path, &self.path).is_err() {
            let _ = fs::remove_file(&temp_path);
        }
    }
}

/// Moves a refused primary to its fixed `.refused` sibling, replacing
/// whatever already sits there — the quarantine slot is fixed, never
/// timestamped (standing ruling), so a stale occupant from an earlier
/// refusal is expected and must not block this one. Skips anything that is
/// not a regular file at `primary_path` (a directory, a FIFO, a character
/// device): those were already refused by `load()`'s own checks and moving
/// them is not what "quarantine a file" means.
fn quarantine_refused_file(primary_path: &Path, quarantine_path: &Path) {
    match fs::metadata(primary_path) {
        Ok(metadata) if metadata.is_file() => {}
        _ => return,
    }
    clear_quarantine_slot(quarantine_path);
    let _ = fs::rename(primary_path, quarantine_path);
}

fn clear_quarantine_slot(path: &Path) {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            let _ = fs::remove_dir_all(path);
        }
        Ok(_) => {
            let _ = fs::remove_file(path);
        }
        Err(_) => {}
    }
}

#[cfg(unix)]
fn open_for_load(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    // @law: opening a FIFO for reading blocks the calling thread until a
    // writer opens the other end; a regular file, a directory, or a
    // character device never blocks on open. `O_NONBLOCK` turns that wait
    // into an immediate return (harmless no-op on the other types), which is
    // what keeps a planted FIFO at the primary path from freezing the UI
    // thread at startup instead of being refused like any other bad file.
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
}

#[cfg(not(unix))]
fn open_for_load(path: &Path) -> std::io::Result<fs::File> {
    fs::File::open(path)
}

#[cfg(unix)]
fn create_owner_only_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    if let Ok(metadata) = fs::symlink_metadata(dir) {
        if metadata.file_type().is_symlink() {
            return Ok(());
        }
        return tighten_to_owner_only(dir, metadata.permissions().mode());
    }
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let mode = fs::symlink_metadata(dir)?.permissions().mode();
    tighten_to_owner_only(dir, mode)
}

/// Removes group/other permission bits when present, and does nothing
/// otherwise — never grants a bit the directory did not already have.
/// Owner bits and any bits above the permission triplet (setuid/setgid/
/// sticky) are left exactly as found.
#[cfg(unix)]
fn tighten_to_owner_only(dir: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if mode & 0o077 == 0 {
        return Ok(());
    }
    fs::set_permissions(dir, fs::Permissions::from_mode(mode & !0o077))
}

#[cfg(not(unix))]
fn create_owner_only_dir(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)
}

#[cfg(unix)]
fn write_owner_only_new_file(path: &Path, payload: &str) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    file.write_all(payload.as_bytes())?;
    file.sync_all()
}

#[cfg(not(unix))]
fn write_owner_only_new_file(path: &Path, payload: &str) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
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
    // cap's worth of bytes: what this pins is "refused before being read",
    // and a sparse file exercises it identically to a dense one.
    // @law: the filesystem reports the same `len()` for a sparse file as
    // for a dense one of the same size — this test relies on that fact to
    // stay cheap on every `cargo test` run.
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

    // @law: `/dev/zero` reports `len() == 0`, like a FIFO, but unlike a FIFO
    // never blocks on open — safe to exercise directly. Unlike `/dev/null`
    // (which delivers zero bytes either way), reading from it actually
    // delivers data past the cap, so this is the one device that would
    // still pass even with `is_file()` and the `.take()` guard both deleted
    // — `/dev/null` alone would not catch that regression.
    #[cfg(unix)]
    #[test]
    fn load_refuses_a_character_device_even_though_its_reported_length_is_zero() {
        let store = FileSnapshotStore::at(PathBuf::from("/dev/zero"));

        assert_eq!(store.load(), None);
    }

    // Reproduces the pre-fix bug: `open_for_load(&self.path).ok()?` exited
    // via `?` on ANY open error, including one that means "the file is
    // there but I could not read it" (EACCES) — routing it around
    // `refuse()` and losing the only copy with no quarantine. Skipped when
    // running as root: root ignores the mode bit entirely and the open
    // would succeed, making this test assert nothing.
    #[cfg(unix)]
    #[test]
    fn an_existing_but_unopenable_primary_is_quarantined_instead_of_silently_dropped() {
        use std::os::unix::fs::PermissionsExt;

        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        fs::write(&path, "unreadable-content").expect("temp file must be writable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000))
            .expect("permissions must be settable");

        let store = FileSnapshotStore::at(path.clone());
        assert_eq!(store.load(), None);

        assert!(
            !path.exists(),
            "expected the unopenable primary moved away, not left in place"
        );
        let quarantined = dir.join("habits.json.refused");
        assert!(
            quarantined.is_file(),
            "expected the unopenable primary preserved at the quarantine sibling"
        );
        // rename() carries the mode along, so the quarantined copy is still
        // 0o000 — restore access before reading it back, exactly like an
        // operator recovering the file would have to.
        fs::set_permissions(&quarantined, fs::Permissions::from_mode(0o600))
            .expect("permissions must be restorable on the quarantined copy");
        assert_eq!(
            fs::read(&quarantined).ok(),
            Some(b"unreadable-content".to_vec()),
            "expected the unreadable primary's bytes preserved at the quarantine sibling"
        );
    }

    #[cfg(unix)]
    #[test]
    fn load_over_a_fifo_refuses_promptly_instead_of_blocking() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        let c_path = std::ffi::CString::new(path.to_str().expect("temp path must be utf-8"))
            .expect("temp path must have no interior nul");
        let mkfifo_result = unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) };
        assert_eq!(
            mkfifo_result, 0,
            "mkfifo must succeed for this test to mean anything"
        );

        let store = FileSnapshotStore::at(path);

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

        let store = FileSnapshotStore::at(path_that_is_a_directory.clone());
        let mut would_be_quarantine_path = path_that_is_a_directory.clone().into_os_string();
        would_be_quarantine_path.push(".refused");

        assert_eq!(store.load(), None);
        assert!(
            path_that_is_a_directory.is_dir(),
            "expected the directory left in place, never quarantined"
        );
        assert!(
            !PathBuf::from(would_be_quarantine_path).exists(),
            "expected no quarantine attempt for a non-file primary"
        );
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

    // B1: `read_to_string` errs with `InvalidData` on non-UTF-8 bytes, and
    // the pre-fix code exited via `.ok()?` without ever quarantining —
    // realistic after an interrupted non-atomic write from an earlier
    // build, since a multi-byte character (e.g. "é") can be truncated
    // mid-sequence.
    #[test]
    fn an_invalid_utf8_primary_file_is_quarantined_instead_of_silently_dropped() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        fs::write(&path, [0x00u8, 0xff, 0xfe, 0x41, 0x42]).expect("temp file must be writable");

        let store = FileSnapshotStore::at(path.clone());
        assert_eq!(store.load(), None);

        assert!(
            !path.exists(),
            "expected the primary moved away, not left in place"
        );
        assert_eq!(
            fs::read(dir.join("habits.json.refused")).ok(),
            Some(vec![0x00u8, 0xff, 0xfe, 0x41, 0x42]),
            "expected the invalid bytes preserved verbatim at the quarantine sibling"
        );
    }

    #[test]
    fn a_refusal_replaces_a_stale_quarantine_directory_instead_of_leaving_the_primary_stuck() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        let file = fs::File::create(&path).expect("temp file must be creatable");
        file.set_len(FileSnapshotStore::MAX_PAYLOAD_BYTES + 1)
            .expect("sparse file must be extendable");
        let stale_quarantine = dir.join("habits.json.refused");
        fs::create_dir_all(&stale_quarantine).expect("stale quarantine dir must be creatable");
        fs::write(stale_quarantine.join("leftover"), "from-a-previous-refusal")
            .expect("stale quarantine content must be writable");

        let store = FileSnapshotStore::at(path.clone());
        assert_eq!(store.load(), None);

        assert!(
            !path.exists(),
            "expected the new refusal to actually move the primary away"
        );
        assert!(
            stale_quarantine.is_file(),
            "expected the stale quarantine directory replaced by the fresh refused file"
        );
        assert_eq!(
            fs::metadata(&stale_quarantine).map(|m| m.len()).ok(),
            Some(FileSnapshotStore::MAX_PAYLOAD_BYTES + 1)
        );
    }

    // A `.refused` slot occupied by a directory (never produced by this
    // code, but user data can end up anywhere) must not be destroyed to
    // make room for a fresh refusal — `remove_dir_all` on it would take a
    // user's manual backups with it. The occupant's own content must
    // survive the eviction in some recoverable form.
    #[test]
    fn a_stale_quarantine_directorys_contents_survive_the_refusal_that_evicts_it() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        let file = fs::File::create(&path).expect("temp file must be creatable");
        file.set_len(FileSnapshotStore::MAX_PAYLOAD_BYTES + 1)
            .expect("sparse file must be extendable");
        let stale_quarantine = dir.join("habits.json.refused");
        fs::create_dir_all(stale_quarantine.join("2026-01"))
            .expect("nested stale content must be creatable");
        fs::write(
            stale_quarantine.join("2026-01").join("backup-a.json"),
            "my-manual-backup",
        )
        .expect("nested stale file must be writable");
        fs::write(stale_quarantine.join("README.txt"), "my manual backups")
            .expect("stale readme must be writable");

        let store = FileSnapshotStore::at(path.clone());
        assert_eq!(store.load(), None);

        assert!(
            stale_quarantine.is_file(),
            "expected the fixed .refused slot to hold the fresh refusal"
        );
        let survivors: Vec<_> = fs::read_dir(&dir)
            .expect("container dir must be readable")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("habits.json.refused.")
            })
            .collect();
        assert_eq!(
            survivors.len(),
            1,
            "expected exactly one evicted copy of the stale occupant, found: {survivors:?}"
        );
        let evicted = survivors[0].path();
        assert!(
            evicted.is_dir(),
            "expected the evicted occupant to still be a directory"
        );
        assert_eq!(
            fs::read_to_string(evicted.join("README.txt")).ok(),
            Some("my manual backups".to_string()),
            "expected the stale occupant's own file preserved verbatim"
        );
        assert_eq!(
            fs::read_to_string(evicted.join("2026-01").join("backup-a.json")).ok(),
            Some("my-manual-backup".to_string()),
            "expected the stale occupant's nested content preserved verbatim"
        );
    }

    // @law: NAME_MAX is 255 bytes on ext4, APFS and tmpfs alike -- a
    // filename just under it accepts a plain write, but the same name plus
    // this store's temp-file suffix crosses it, failing every attempt with
    // `ENAMETOOLONG` identically and deterministically.
    #[test]
    fn a_failed_temp_write_leaves_the_previous_snapshot_intact() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let name_right_under_the_name_length_limit = "h".repeat(240);
        let path = dir.join(&name_right_under_the_name_length_limit);
        fs::write(&path, "previous-payload").expect("temp file must be writable");
        let store = FileSnapshotStore::at(path.clone());

        store.save("payload-that-must-not-land");

        assert_eq!(
            fs::read_to_string(&path).ok(),
            Some("previous-payload".to_string()),
            "expected a failed write to leave the previous snapshot untouched"
        );
    }

    #[test]
    fn a_failed_rename_leaves_no_temp_file_behind() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("habits.json");
        fs::create_dir_all(&path).expect("path-as-directory must be creatable");
        fs::write(path.join("occupant"), "keep-me").expect("occupant must be writable");
        let store = FileSnapshotStore::at(path);

        store.save("payload-that-cannot-land");

        let leftover_temp_files: Vec<_> = fs::read_dir(&dir)
            .expect("container dir must be readable")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(
            leftover_temp_files.is_empty(),
            "expected no temp file left behind after a failed rename, found: {leftover_temp_files:?}"
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

    #[cfg(unix)]
    #[test]
    fn save_tightens_a_pre_existing_directory_to_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755))
            .expect("permissions must be settable");
        let path = dir.join("habits.json");

        FileSnapshotStore::at(path).save("payload");

        let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            dir_mode, 0o700,
            "expected a pre-existing directory tightened to owner-only"
        );
    }

    // A directory deliberately protected by the user at 0o500 (no write
    // bit) must stay exactly as protected, not get "tightened" into
    // 0o700 — which would silently grant this app write access the user
    // never gave it. mode & 0o077 is already 0 here (no group/other bits),
    // so nothing should be touched at all.
    #[cfg(unix)]
    #[test]
    fn save_never_loosens_a_directory_protected_below_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o500))
            .expect("permissions must be settable");
        let path = dir.join("habits.json");

        FileSnapshotStore::at(path).save("payload");

        let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            dir_mode, 0o500,
            "expected a directory protected below owner-only left exactly as the user set it"
        );
    }

    // A parent directory that is itself a symlink to a directory shared
    // with other things (e.g. a backup tool) must never have its target's
    // mode rewritten — only the final path component is inspected, and a
    // symlink there means "do nothing to the permissions", not "follow and
    // tighten what it points at".
    #[cfg(unix)]
    #[test]
    fn save_does_not_change_the_mode_of_a_symlinked_parents_target() {
        use std::os::unix::fs::PermissionsExt;

        let target = unused_temp_path();
        fs::create_dir_all(&target).expect("target dir must be creatable");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
            .expect("permissions must be settable");
        fs::write(target.join("thesis.txt"), "shared-content")
            .expect("shared file must be writable");
        let link_container = unused_temp_path();
        fs::create_dir_all(&link_container).expect("link container must be creatable");
        let link = link_container.join("data");
        std::os::unix::fs::symlink(&target, &link).expect("symlink must be creatable");
        let path = link.join("habits.json");

        FileSnapshotStore::at(path).save("payload");

        let target_mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            target_mode, 0o755,
            "expected the symlink's target left at its original, shared mode"
        );
        assert!(
            target.join("thesis.txt").exists(),
            "expected the unrelated shared file left untouched"
        );
    }

    #[test]
    fn a_successful_save_leaves_no_temp_file_behind() {
        let dir = unused_temp_path();
        let path = dir.join("habits.json");
        let store = FileSnapshotStore::at(path);

        store.save("payload");

        let leftover_temp_files: Vec<_> = fs::read_dir(&dir)
            .expect("save must have created the directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(
            leftover_temp_files.is_empty(),
            "expected no temp file left behind after a successful save, found: {leftover_temp_files:?}"
        );
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

    #[cfg(unix)]
    #[test]
    fn write_owner_only_new_file_refuses_a_pre_existing_regular_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let path = dir.join("temp-target");
        fs::write(&path, "planted").expect("planted file must be writable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666))
            .expect("permissions must be settable");

        let result = write_owner_only_new_file(&path, "attacker-payload");

        assert!(
            result.is_err(),
            "expected create_new to refuse an already-occupied target"
        );
        assert_eq!(
            fs::read_to_string(&path).ok(),
            Some("planted".to_string()),
            "expected the pre-existing file left completely untouched"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_owner_only_new_file_refuses_to_follow_a_symlink_to_a_victim() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let victim = dir.join("victim");
        fs::write(&victim, "victim-content").expect("victim must be writable");
        let link = dir.join("link");
        std::os::unix::fs::symlink(&victim, &link).expect("symlink must be creatable");

        let result = write_owner_only_new_file(&link, "attacker-payload");

        assert!(result.is_err(), "expected the symlink write to be refused");
        assert_eq!(
            fs::read_to_string(&victim).ok(),
            Some("victim-content".to_string()),
            "expected the symlink's target left completely untouched"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_owner_only_new_file_refuses_a_dangling_symlink_instead_of_materialising_it() {
        let dir = unused_temp_path();
        fs::create_dir_all(&dir).expect("temp dir must be creatable");
        let nonexistent_target = dir.join("does-not-exist");
        let link = dir.join("dangling-link");
        std::os::unix::fs::symlink(&nonexistent_target, &link)
            .expect("dangling symlink must be creatable");

        let result = write_owner_only_new_file(&link, "attacker-payload");

        assert!(
            result.is_err(),
            "expected the dangling symlink write to be refused"
        );
        assert!(
            !nonexistent_target.exists(),
            "expected no file materialised at the symlink's target"
        );
    }
}
