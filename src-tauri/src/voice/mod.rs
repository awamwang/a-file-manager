// SPDX-License-Identifier: GPL-3.0-or-later
//! 语音输入套件（ADR 0018）：Rust 采集 + OpenAI 兼容批量转写。
//!
//! 单飞会话：同一时刻最多一路录音。命令只做会话管理；
//! 识别链路 = cpal 采集 → 线性重采样 16k mono → hound 编 WAV
//! → ureq 手工 multipart 上传 → 事件回前端（`voice://*`）。
//! 接入点（搜索面板）只消费事件，不感知引擎与网络协议。

pub mod capture;
pub mod engine;

use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// 前端监听的事件名。
pub const STATE_EVENT: &str = "voice://state";
pub const TRANSCRIPT_EVENT: &str = "voice://transcript";
pub const ERROR_EVENT: &str = "voice://error";

/// 转写结果的去向标签：`palette`（搜索面板接入点）| `selftest`（语音自检）。
pub const SOURCE_PALETTE: &str = "palette";
pub const SOURCE_SELFTEST: &str = "selftest";

/// 录音上限：到时自动走 stop 路径（转写正常出文本）。
pub const MAX_RECORDING_SECS: f32 = 60.0;
/// 低于该时长的录音视为误触，返回 `empty` 不上传。
pub const MIN_RECORDING_SECS: f32 = 0.3;

// ---------------------------------------------------------------------------
// 事件 payload（camelCase，与 file-ops-progress 惯例一致）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VoiceStateEvent {
    /// `recording` | `transcribing` | `idle`
    pub state: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptEvent {
    pub text: String,
    pub source: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VoiceErrorEvent {
    pub code: String,
    pub source: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// 错误：命令同步失败走 invoke 拒绝；异步（上传）失败走 `voice://error` 事件。
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VoiceError {
    pub code: String,
    pub message: String,
    pub status: u16,
}

impl VoiceError {
    pub fn new(code: &str, message: impl Into<String>, status: u16) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            status,
        }
    }

    pub fn busy() -> Self {
        Self::new("busy", "a voice recording is already active", 0)
    }

    pub fn no_device() -> Self {
        Self::new("no-device", "no microphone input device available", 0)
    }

    pub fn permission(message: impl Into<String>) -> Self {
        Self::new("permission", message, 0)
    }

    pub fn not_configured() -> Self {
        Self::new(
            "not-configured",
            "stt base URL / model / API key must be configured first",
            0,
        )
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::new("network", message, 0)
    }

    pub fn unauthorized(status: u16) -> Self {
        Self::new("unauthorized", "transcription service rejected the API key", status)
    }

    pub fn server(message: impl Into<String>, status: u16) -> Self {
        Self::new("server", message, status)
    }
}

impl std::fmt::Display for VoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "voice error [{}]: {}", self.code, self.message)
    }
}

// ---------------------------------------------------------------------------
// 会话：静态单飞；Stream 由采集线程持有，会话只握控制通道。
// ---------------------------------------------------------------------------

struct VoiceSession {
    samples: std::sync::Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    source: String,
    stop_tx: std::sync::mpsc::Sender<()>,
    done_rx: std::sync::mpsc::Receiver<()>,
    started_at: Instant,
}

static SESSION: Mutex<Option<VoiceSession>> = Mutex::new(None);

fn emit_state(app: &AppHandle, state: &str, source: &str) {
    let _ = app.emit(
        STATE_EVENT,
        VoiceStateEvent {
            state: state.to_string(),
            source: source.to_string(),
        },
    );
}

fn emit_error(app: &AppHandle, code: &str, source: &str, message: impl Into<String>) {
    let _ = app.emit(
        ERROR_EVENT,
        VoiceErrorEvent {
            code: code.to_string(),
            source: source.to_string(),
            message: message.into(),
        },
    );
}

/// 开启一路录音。已在录音 → `busy`；配置缺失 → `not-configured`（快速失败）。
fn start_session(app: &AppHandle, source: &str) -> Result<(), VoiceError> {
    // 快速失败：麦克风点下去立刻知道配置缺失，而不是录完才发现。
    engine::read_config(app)?;
    let mut guard = SESSION.lock().map_err(|_| VoiceError::busy())?;
    if guard.is_some() {
        return Err(VoiceError::busy());
    }
    let handle = capture::spawn_capture().map_err(|err| match err {
        capture::CaptureError::NoDevice => VoiceError::no_device(),
        capture::CaptureError::Permission(message) => VoiceError::permission(message),
    })?;
    *guard = Some(VoiceSession {
        samples: handle.samples,
        sample_rate: handle.sample_rate,
        source: source.to_string(),
        stop_tx: handle.stop_tx,
        done_rx: handle.done_rx,
        started_at: Instant::now(),
    });
    drop(guard);
    emit_state(app, "recording", source);
    // 60s 自动截止监视：到点走 finalize(false)，正常出文本。
    let monitor_app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis((MAX_RECORDING_SECS * 1000.0) as u64 + 250));
        let _ = finalize(&monitor_app, false);
    });
    Ok(())
}

/// 收尾：take 会话 → 停采集 → cancel 丢弃 / 否则转写上传。
/// 会话不存在时静默成功（重复 stop / 迟到的自动截止都无害）。
fn finalize(app: &AppHandle, cancel: bool) -> Result<(), VoiceError> {
    let session = SESSION
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    let Some(session) = session else {
        return Ok(());
    };
    // 停采集线程并等它落停，避免采样缓冲被并发写入。
    let _ = session.stop_tx.send(());
    let _ = session.done_rx.recv_timeout(Duration::from_secs(2));
    if cancel {
        emit_state(app, "idle", &session.source);
        return Ok(());
    }
    let raw = session
        .samples
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    let min_samples = (session.sample_rate as f32 * MIN_RECORDING_SECS) as usize;
    if raw.len() < min_samples {
        emit_error(app, "empty", &session.source, "recording too short");
        emit_state(app, "idle", &session.source);
        return Ok(());
    }
    emit_state(app, "transcribing", &session.source);
    let handle = app.clone();
    let source = session.source.clone();
    let started_at = session.started_at;
    let sample_rate = session.sample_rate;
    std::thread::spawn(move || {
        let samples = engine::resample_to_16k_mono(&raw, sample_rate);
        let wav = match engine::encode_wav(&samples) {
            Ok(wav) => wav,
            Err(message) => {
                emit_error(&handle, "server", &source, message);
                emit_state(&handle, "idle", &source);
                return;
            }
        };
        let config = match engine::read_config(&handle) {
            Ok(config) => config,
            Err(err) => {
                emit_error(&handle, &err.code, &source, err.message);
                emit_state(&handle, "idle", &source);
                return;
            }
        };
        match engine::transcribe(&config, &wav) {
            Ok(text) => {
                let _ = handle.emit(
                    TRANSCRIPT_EVENT,
                    VoiceTranscriptEvent {
                        text,
                        source: source.clone(),
                        elapsed_ms: started_at.elapsed().as_millis() as u64,
                    },
                );
                emit_state(&handle, "idle", &source);
            }
            Err(err) => {
                emit_error(&handle, &err.code, &source, err.message);
                emit_state(&handle, "idle", &source);
            }
        }
    });
    Ok(())
}

/// 搜索面板接入点：开始录音（`voice://transcript` 的 source 为 `palette`）。
#[tauri::command]
pub fn voice_start(app: AppHandle) -> Result<(), VoiceError> {
    start_session(&app, SOURCE_PALETTE)
}

/// 语音自检（ADR 0018）：source 为 `selftest`，供设置面板试录回显。
#[tauri::command]
pub fn voice_selftest(app: AppHandle) -> Result<(), VoiceError> {
    start_session(&app, SOURCE_SELFTEST)
}

/// 停止录音并转写：`recording → transcribing → (transcript|error) → idle`。
#[tauri::command]
pub fn voice_stop(app: AppHandle) -> Result<(), VoiceError> {
    finalize(&app, false)
}

/// 取消录音：不出文本，直接回 `idle`。
#[tauri::command]
pub fn voice_cancel(app: AppHandle) -> Result<(), VoiceError> {
    finalize(&app, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_event_serializes_camel_case() {
        let event = VoiceStateEvent {
            state: "recording".to_string(),
            source: "palette".to_string(),
        };
        let json = serde_json::to_string(&event).expect("serialize state event");
        assert!(json.contains("\"state\":\"recording\""));
        assert!(json.contains("\"source\":\"palette\""));
    }

    #[test]
    fn transcript_event_serializes_camel_case() {
        let event = VoiceTranscriptEvent {
            text: "hello".to_string(),
            source: "selftest".to_string(),
            elapsed_ms: 1234,
        };
        let json = serde_json::to_string(&event).expect("serialize transcript event");
        assert!(json.contains("\"elapsedMs\":1234"));
        assert!(json.contains("\"text\":\"hello\""));
    }

    #[test]
    fn error_serializes_camel_case_and_codes_are_stable() {
        let error = VoiceError::not_configured();
        assert_eq!(error.code, "not-configured");
        let json = serde_json::to_string(&error).expect("serialize error");
        assert!(json.contains("\"code\":\"not-configured\""));
        assert_eq!(VoiceError::busy().code, "busy");
        assert_eq!(VoiceError::no_device().code, "no-device");
        assert_eq!(VoiceError::network("x").code, "network");
        assert_eq!(VoiceError::unauthorized(401).code, "unauthorized");
        assert_eq!(VoiceError::server("x", 500).code, "server");
        assert_eq!(VoiceError::permission("x").code, "permission");
    }
}
