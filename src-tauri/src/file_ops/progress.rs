// SPDX-License-Identifier: GPL-3.0-or-later
//! 长任务进度事件（右下角任务列表条）：copy/move 按 taskId 上报字节进度。
//! 事件经节流后发给前端（默认 80ms 一次），避免大文件复制刷爆 IPC。

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// 前端监听的事件名：`file-ops-progress`。
pub const FILE_OPS_PROGRESS_EVENT: &str = "file-ops-progress";

/// 进度事件节流间隔（毫秒）。
const EMIT_INTERVAL_MS: u128 = 80;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOpsProgressPayload {
    pub task_id: String,
    /// `start` | `progress` | `done`
    pub phase: &'static str,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub ok: bool,
    pub error: Option<String>,
}

/// 单次 invoke 内的进度发射器；跨 invoke 复用同一 taskId 时由前端累加。
pub struct ProgressEmitter {
    app: AppHandle,
    task_id: String,
    bytes_total: u64,
    bytes_done: AtomicU64,
    last_progress_emit: Mutex<Option<Instant>>,
}

impl ProgressEmitter {
    pub fn new(app: AppHandle, task_id: String, bytes_total: u64) -> Self {
        Self {
            app,
            task_id,
            bytes_total,
            bytes_done: AtomicU64::new(0),
            last_progress_emit: Mutex::new(None),
        }
    }

    fn emit(&self, payload: FileOpsProgressPayload) {
        let _ = self.app.emit(FILE_OPS_PROGRESS_EVENT, payload);
    }

    pub fn emit_start(&self) {
        *self.last_progress_emit.lock().expect("progress emitter lock") = None;
        self.emit(FileOpsProgressPayload {
            task_id: self.task_id.clone(),
            phase: "start",
            bytes_done: 0,
            bytes_total: self.bytes_total,
            ok: true,
            error: None,
        });
    }

    /// 复制线程回调：累计字节并按 EMIT_INTERVAL_MS 节流上报。
    pub fn on_bytes(&self, count: u64) {
        let done = self.bytes_done.fetch_add(count, Ordering::Relaxed) + count;
        let mut last = self.last_progress_emit.lock().expect("progress emitter lock");
        let now = Instant::now();
        if let Some(previous) = *last {
            if now.duration_since(previous).as_millis() < EMIT_INTERVAL_MS {
                return;
            }
        }
        *last = Some(now);
        drop(last);
        self.emit(FileOpsProgressPayload {
            task_id: self.task_id.clone(),
            phase: "progress",
            bytes_done: done,
            bytes_total: self.bytes_total,
            ok: true,
            error: None,
        });
    }

    pub fn emit_done(&self, ok: bool, error: Option<String>) {
        let done = self.bytes_done.load(Ordering::Relaxed);
        self.emit(FileOpsProgressPayload {
            task_id: self.task_id.clone(),
            phase: "done",
            bytes_done: done,
            bytes_total: self.bytes_total,
            ok,
            error,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_serializes_camel_case() {
        let payload = FileOpsProgressPayload {
            task_id: "t-1".into(),
            phase: "start",
            bytes_done: 0,
            bytes_total: 128,
            ok: true,
            error: None,
        };
        let json = serde_json::to_string(&payload).expect("serialize payload");
        assert!(json.contains("\"taskId\":\"t-1\""));
        assert!(json.contains("\"bytesTotal\":128"));
        assert!(json.contains("\"phase\":\"start\""));
    }
}
