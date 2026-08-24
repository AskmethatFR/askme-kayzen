//! Android's `Context.getFilesDir()` reached over JNI, and nothing else.
//!
//! This module holds no policy (C2, adr-0016 AD-1): it does not check
//! whether the returned path is empty or relative, and it does not append
//! `kayzen` to it. Every such rule stays in `resolve_data_dir`, in the
//! composition root, where it applies to every platform and is testable on
//! a host. This module's only job is answering the one question a host
//! cannot answer for itself: what does the JVM say `getFilesDir()` is.
//!
//! # Precondition
//! @law: relies on `std::panic::catch_unwind` turning a panic into `None`
//! rather than aborting the process. That conversion only happens under the
//! `unwind` panic strategy, which is this workspace's unconfigured default
//! (owner ruling OQ-1: document the precondition here, do not pin
//! `panic = "unwind"` in any `[profile]`).
//!
//! # Threading
//! @law: tao initialises the JNI context on its own thread and then
//! `thread::spawn`s a *separate*, unattached thread to run Rust's `main`
//! (`tao-0.34.8/src/platform_impl/android/ndk_glue.rs:255-275`). The
//! Dioxus thread this function runs on is therefore never attached to the
//! JVM by tao itself, so attaching here is required, not optional — and it
//! must stay `attach_current_thread_permanently`, never the guard-returning
//! `attach_current_thread`: the guard detaches on drop, which would
//! invalidate the local references wry holds on this same thread.

use std::path::PathBuf;

use jni::JavaVM;
use jni::errors::Error as JniError;
use jni::objects::{JObject, JString};

/// The app's private files directory, or `None` if any step of reaching it
/// fails. Total — never panics, never falls back to another location, never
/// `unwrap`s.
pub fn files_dir() -> Option<PathBuf> {
    let context = std::panic::catch_unwind(ndk_context::android_context).ok()?;

    let vm_ptr = context.vm();
    let activity_ptr = context.context();
    if vm_ptr.is_null() || activity_ptr.is_null() {
        eprintln!("android_files_dir: android context has a null vm or activity pointer");
        return None;
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }.ok()?;
    let mut env = vm.attach_current_thread_permanently().ok()?;

    // @law: `activity_ptr` is tao's own global ref to the activity
    // (`ndk_glue.rs`, `activity.as_obj().as_raw()`). It is wrapped here only
    // to call methods on it and must never be deleted through this
    // local-ref-shaped handle — freeing a global ref through a local-ref API
    // is undefined behaviour, and this function only ever reads through it.
    let activity = unsafe { JObject::from_raw(activity_ptr.cast()) };

    let outcome = env.with_local_frame(8, |env| files_dir_absolute_path(env, &activity));

    match outcome {
        Ok(path) => Some(PathBuf::from(path)),
        Err(_) => {
            // @law: jni 0.21's `Err(JavaException)` does not clear the
            // pending exception (jni-0.21.1/src/wrapper/macros.rs:84). The
            // JVM aborts the process on the next JNI call made with one
            // still pending, and wry makes such calls constantly — so a
            // failure here must be cleared before returning `None`, or a
            // refusal screen becomes a crash.
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_describe();
                let _ = env.exception_clear();
            }
            None
        }
    }
}

/// `Context.getFilesDir().getAbsolutePath()`, run inside the caller's local
/// reference frame. Never logs the resolved path (no PII in a diagnostic
/// that tao pipes straight to logcat) — only which step failed, and only on
/// the failing step.
fn files_dir_absolute_path(
    env: &mut jni::JNIEnv<'_>,
    activity: &JObject<'_>,
) -> Result<String, JniError> {
    let files_dir = env
        .call_method(activity, "getFilesDir", "()Ljava/io/File;", &[])
        .inspect_err(|_| eprintln!("android_files_dir: Context.getFilesDir() failed"))?
        .l()?;

    let absolute_path = env
        .call_method(&files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .inspect_err(|_| eprintln!("android_files_dir: File.getAbsolutePath() failed"))?
        .l()?;

    let java_string = JString::from(absolute_path);
    let java_str = env
        .get_string(&java_string)
        .inspect_err(|_| eprintln!("android_files_dir: reading the returned Java string failed"))?;

    Ok(String::from(java_str))
}
