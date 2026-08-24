#[cfg(not(target_arch = "wasm32"))]
pub mod file_snapshot_store;

#[cfg(target_os = "android")]
pub mod android_files_dir;

#[cfg(target_arch = "wasm32")]
pub mod local_storage_snapshot_store;
