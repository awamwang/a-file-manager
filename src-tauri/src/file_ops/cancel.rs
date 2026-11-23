// SPDX-License-Identifier: GPL-3.0-or-later
//! Cancellation registry for long file operations (LC-005).
//!
//! A copy/move that runs as a tracked long task registers its `taskId` here and
//! polls the token between top-level sources and between merge children. The
//! front-end's Cancel button calls the `file_ops_cancel` command, which flips the
//! shared flag. On cancel the transfer loop stops and rolls back the work this
//! call has committed (copy → delete created items; move → move items back).
//!
//! Rollback is deliberately **scoped** (2026-11-22 decision):
//! - plain copy / plain move: per-item rollback (delete created / move back).
//! - directory **merge** and cross-volume moves still in flight: safe stop only.
//!   `safe_io` guarantees no source loss and no half-copied file, but unwinding a
//!   partially-merged folder risks the target's pre-existing contents, so we do
//!   not attempt to un-merge. The task is reported as cancelled, not failed.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

fn registry() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// RAII handle: registration is dropped (and the entry removed) when the owning
/// transfer finishes, so a completed task's id can never be cancelled by mistake.
pub struct CancelToken {
    flag: Arc<AtomicBool>,
    task_id: String,
}

impl CancelToken {
    pub fn cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}

impl Drop for CancelToken {
    fn drop(&mut self) {
        if let Ok(mut map) = registry().lock() {
            // Remove only our own entry, keyed by the same flag instance.
            if let Some(existing) = map.get(&self.task_id) {
                if Arc::ptr_eq(existing, &self.flag) {
                    map.remove(&self.task_id);
                }
            }
        }
    }
}

/// Register a fresh cancel flag for `task_id`. Overwrites any stale entry.
pub fn register(task_id: &str) -> CancelToken {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut map) = registry().lock() {
        map.insert(task_id.to_string(), Arc::clone(&flag));
    }
    CancelToken {
        flag,
        task_id: task_id.to_string(),
    }
}

/// Flip the flag for `task_id`. Returns `true` if a matching running task exists
/// (i.e. the cancel was accepted), `false` if the id is unknown / already done.
pub fn request_cancel(task_id: &str) -> bool {
    if let Ok(map) = registry().lock() {
        if let Some(flag) = map.get(task_id) {
            flag.store(true, Ordering::Relaxed);
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_reflects_request_and_unregisters_on_drop() {
        let token = register("cancel-test-1");
        assert!(!token.cancelled());
        assert!(request_cancel("cancel-test-1"), "known id accepts cancel");
        assert!(token.cancelled(), "flag observed by the owner");
        drop(token);
        // After drop the id is gone → a later cancel is a no-op.
        assert!(!request_cancel("cancel-test-1"));
    }

    #[test]
    fn cancel_isolates_tasks_by_id() {
        let a = register("cancel-test-a");
        let b = register("cancel-test-b");
        request_cancel("cancel-test-a");
        assert!(a.cancelled());
        assert!(!b.cancelled(), "cancelling one task must not cancel another");
    }
}
