#[cfg(not(target_arch = "wasm32"))]
pub mod file_snapshot_store;

#[cfg(target_arch = "wasm32")]
pub mod local_storage_snapshot_store;
