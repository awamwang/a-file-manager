// SPDX-License-Identifier: GPL-3.0-or-later

use super::provider::Provider;
use super::types::{
    AwamUri, BackendInfo, DirListing, ProviderCapability, ProviderEntry, ResolveResult, StatResult,
};
use crate::win_process;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const CAPABILITIES: &[ProviderCapability] = &[
    ProviderCapability::List,
    ProviderCapability::Stat,
    ProviderCapability::Resolve,
];

/// listremotes 本地读配置，短超时即可。
const LIST_REMOTES_TIMEOUT: Duration = Duration::from_secs(15);
/// 进入具体 remote（含 SSH）的墙钟上限；超时必须返回错误，禁止无限挂起。
const LIST_REMOTE_TIMEOUT: Duration = Duration::from_secs(20);
/// 杀进程后等待管道线程退出的上限；超时放弃 join，避免卡死。
const PIPE_JOIN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait RcloneRunner: Send + Sync {
    fn run(&self, args: &[&str], timeout: Duration) -> Result<CommandOutput, String>;
}

/// rclone CLI runner (no mount / no VFS). Honors `timeout` by killing the child tree.
#[derive(Debug)]
pub struct BlockingRcloneRunner {
    /// 已解析的可执行文件绝对路径；未安装时为 `None`，避免盲目 spawn。
    pub exe: Option<PathBuf>,
    /// 探测失败时的用户可读说明（健康检查 / 列表直接展示）。
    pub missing_message: Option<String>,
}

impl Default for BlockingRcloneRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockingRcloneRunner {
    pub fn new() -> Self {
        match resolve_rclone_executable() {
            Ok(exe) => Self {
                exe: Some(exe),
                missing_message: None,
            },
            Err(message) => Self {
                exe: None,
                missing_message: Some(message),
            },
        }
    }
}

/// 按平台解析 rclone 可执行文件：环境变量优先，再 PATH（Windows 含 `.exe`）。
fn resolve_rclone_executable() -> Result<PathBuf, String> {
    let explicit = std::env::var("AWAM_RCLONE_EXE")
        .or_else(|_| std::env::var("RCLONE_EXE"))
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    if let Some(candidate) = explicit {
        let as_path = PathBuf::from(&candidate);
        if as_path.is_file() {
            return Ok(as_path);
        }
        if let Some(found) = win_process::resolve_on_path(&candidate) {
            return Ok(found);
        }
        return Err(rclone_missing_message(Some(&candidate)));
    }

    for name in rclone_path_candidates() {
        if let Some(found) = win_process::resolve_on_path(name) {
            return Ok(found);
        }
    }
    Err(rclone_missing_message(None))
}

fn rclone_path_candidates() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &["rclone.exe", "rclone"]
    }
    #[cfg(not(windows))]
    {
        &["rclone"]
    }
}

fn rclone_missing_message(configured: Option<&str>) -> String {
    let hint = rclone_install_hint();
    match configured {
        Some(path) => format!(
            "未找到 rclone（已配置：{path}）。请确认路径有效，或安装后加入 PATH。{hint}"
        ),
        None => format!("未检测到 rclone。云盘 / SFTP / WebDAV 等需先安装 rclone。{hint}"),
    }
}

fn rclone_install_hint() -> &'static str {
    #[cfg(windows)]
    {
        " Windows：从 https://rclone.org/downloads/ 安装，或 `winget install Rclone.Rclone`，也可设置 AWAM_RCLONE_EXE 指向 rclone.exe。"
    }
    #[cfg(target_os = "macos")]
    {
        " macOS：可用 `brew install rclone`，或设置 AWAM_RCLONE_EXE。"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        " Linux：用发行版包管理器安装，或从 https://rclone.org/downloads/ 获取，也可设置 AWAM_RCLONE_EXE。"
    }
    #[cfg(all(not(windows), not(unix)))]
    {
        " 详见 https://rclone.org/downloads/ ，或设置 AWAM_RCLONE_EXE。"
    }
}

fn join_bytes_with_timeout(
    handle: thread::JoinHandle<Vec<u8>>,
    timeout: Duration,
) -> Vec<u8> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(handle.join().unwrap_or_default());
    });
    match rx.recv_timeout(timeout) {
        Ok(buf) => buf,
        Err(_) => Vec::new(),
    }
}

fn terminate_child_tree(child: &mut Child) {
    let pid = child.id();
    win_process::kill_process_tree(pid);
    let _ = child.kill();
    let _ = child.wait();
}

impl RcloneRunner for BlockingRcloneRunner {
    fn run(&self, args: &[&str], timeout: Duration) -> Result<CommandOutput, String> {
        let exe = self.exe.as_ref().ok_or_else(|| {
            self.missing_message
                .clone()
                .unwrap_or_else(|| rclone_missing_message(None))
        })?;
        // rclone is a console binary; CREATE_NO_WINDOW prevents flash on GUI builds
        // (provider_preload / health / listremotes at app start and on Providers refresh).
        // RCLONE_ASK_PASSWORD=false：禁止交互式要密码把管道挂死。
        let mut child = win_process::command(exe)
            .args(args)
            .env("RCLONE_ASK_PASSWORD", "false")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    rclone_missing_message(Some(&exe.display().to_string()))
                } else {
                    format!("无法启动 rclone：{err}")
                }
            })?;

        let mut stdout_pipe = child
            .stdout
            .take()
            .ok_or_else(|| "rclone stdout pipe missing".to_string())?;
        let mut stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| "rclone stderr pipe missing".to_string())?;
        let stdout_thr = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout_pipe.read_to_end(&mut buf);
            buf
        });
        let stderr_thr = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buf);
            buf
        });

        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => {
                    terminate_child_tree(&mut child);
                    let _ = join_bytes_with_timeout(stdout_thr, PIPE_JOIN_TIMEOUT);
                    let _ = join_bytes_with_timeout(stderr_thr, PIPE_JOIN_TIMEOUT);
                    return Err(format!(
                        "rclone timed out after {}s (remote may be unreachable or waiting for credentials)",
                        timeout.as_secs().max(1)
                    ));
                }
                Ok(None) => thread::sleep(Duration::from_millis(25)),
                Err(err) => {
                    terminate_child_tree(&mut child);
                    let _ = join_bytes_with_timeout(stdout_thr, PIPE_JOIN_TIMEOUT);
                    let _ = join_bytes_with_timeout(stderr_thr, PIPE_JOIN_TIMEOUT);
                    return Err(format!("Failed to wait for rclone: {err}"));
                }
            }
        };

        let stdout =
            String::from_utf8_lossy(&join_bytes_with_timeout(stdout_thr, PIPE_JOIN_TIMEOUT))
                .to_string();
        let stderr =
            String::from_utf8_lossy(&join_bytes_with_timeout(stderr_thr, PIPE_JOIN_TIMEOUT))
                .to_string();
        Ok(CommandOutput {
            status_code: status.code().unwrap_or(-1),
            stdout,
            stderr,
        })
    }
}

pub struct RcloneProvider {
    runner: Arc<dyn RcloneRunner>,
}

impl RcloneProvider {
    pub fn new(runner: Arc<dyn RcloneRunner>) -> Self {
        Self { runner }
    }

    pub fn system_default() -> Self {
        Self::new(Arc::new(BlockingRcloneRunner::new()))
    }

    fn list_remotes(&self) -> Result<Vec<String>, String> {
        let output = self
            .runner
            .run(&["listremotes"], LIST_REMOTES_TIMEOUT)?;
        if output.status_code != 0 {
            let err = output.stderr.trim();
            return Err(if err.is_empty() {
                "rclone listremotes failed".to_string()
            } else {
                err.to_string()
            });
        }
        Ok(output
            .stdout
            .lines()
            .map(|line| line.trim().trim_end_matches(':').to_string())
            .filter(|line| !line.is_empty())
            .collect())
    }

    fn remote_spec(uri: &AwamUri) -> Option<String> {
        if uri.path.is_empty() {
            return None;
        }
        let mut parts = uri.path.splitn(2, '/');
        let remote = parts.next()?;
        let rest = parts.next().unwrap_or("");
        Some(format!("{remote}:{rest}"))
    }

    /// 进入具体 remote 时附加网络超时参数，避免 SSH 握手无限挂起。
    fn lsjson_args(spec: &str) -> Vec<&str> {
        vec![
            "lsjson",
            "--max-depth",
            "1",
            "--contimeout",
            "8s",
            "--timeout",
            "15s",
            "--retries",
            "1",
            "--low-level-retries",
            "1",
            spec,
        ]
    }
}

impl Provider for RcloneProvider {
    fn backend_id(&self) -> &str {
        "rclone"
    }
    fn name(&self) -> &str {
        "rclone"
    }
    fn description(&self) -> &str {
        "云盘 / SFTP / WebDAV（本机 rclone，无需挂载）"
    }
    fn capabilities(&self) -> &[ProviderCapability] {
        CAPABILITIES
    }

    fn health(&self) -> BackendInfo {
        match self.list_remotes() {
            Ok(remotes) => BackendInfo {
                id: "rclone".to_string(),
                name: self.name().to_string(),
                description: self.description().to_string(),
                healthy: true,
                capabilities: CAPABILITIES
                    .iter()
                    .map(|capability| capability.as_str().to_string())
                    .collect(),
                detail: Some(if remotes.is_empty() {
                    "尚未配置 remote，可在终端运行 rclone config".to_string()
                } else {
                    format!("已配置 {} 个 remote", remotes.len())
                }),
                interface_only: false,
                source: Some("builtin".to_string()),
            },
            Err(err) => BackendInfo {
                id: "rclone".to_string(),
                name: self.name().to_string(),
                description: self.description().to_string(),
                healthy: false,
                capabilities: CAPABILITIES
                    .iter()
                    .map(|capability| capability.as_str().to_string())
                    .collect(),
                detail: Some(err),
                interface_only: false,
                source: Some("builtin".to_string()),
            },
        }
    }

    fn list(&self, uri: &AwamUri) -> DirListing {
        if uri.path.is_empty() {
            return match self.list_remotes() {
                Ok(remotes) if remotes.is_empty() => DirListing {
                    uri: uri.to_string_uri(),
                    backend: "rclone".to_string(),
                    entries: Vec::new(),
                    error: Some(
                        "尚未配置 rclone remote。请在终端运行 rclone config 添加后再试。"
                            .to_string(),
                    ),
                },
                Ok(remotes) => {
                    let entries = remotes
                        .into_iter()
                        .map(|name| {
                            let mut extra = HashMap::new();
                            extra.insert("virtual".to_string(), Value::Bool(true));
                            ProviderEntry {
                                uri: uri.child(&name).to_string_uri(),
                                name,
                                is_dir: true,
                                is_file: false,
                                size: 0,
                                modified_time: 0,
                                local_path: None,
                                extra,
                            }
                        })
                        .collect();
                    DirListing {
                        uri: uri.to_string_uri(),
                        backend: "rclone".to_string(),
                        entries,
                        error: None,
                    }
                }
                Err(err) => DirListing {
                    uri: uri.to_string_uri(),
                    backend: "rclone".to_string(),
                    entries: Vec::new(),
                    error: Some(err),
                },
            };
        }

        let Some(spec) = Self::remote_spec(uri) else {
            return DirListing {
                uri: uri.to_string_uri(),
                backend: "rclone".to_string(),
                entries: Vec::new(),
                error: Some("Invalid rclone URI".to_string()),
            };
        };

        let args = Self::lsjson_args(&spec);
        match self.runner.run(&args, LIST_REMOTE_TIMEOUT) {
            Ok(output) if output.status_code == 0 => match parse_lsjson(&output.stdout, uri) {
                Ok(entries) => DirListing {
                    uri: uri.to_string_uri(),
                    backend: "rclone".to_string(),
                    entries,
                    error: None,
                },
                Err(err) => DirListing {
                    uri: uri.to_string_uri(),
                    backend: "rclone".to_string(),
                    entries: Vec::new(),
                    error: Some(err),
                },
            },
            Ok(output) => {
                let err = output.stderr.trim();
                DirListing {
                    uri: uri.to_string_uri(),
                    backend: "rclone".to_string(),
                    entries: Vec::new(),
                    error: Some(if err.is_empty() {
                        "rclone lsjson failed".to_string()
                    } else {
                        err.to_string()
                    }),
                }
            }
            Err(err) => DirListing {
                uri: uri.to_string_uri(),
                backend: "rclone".to_string(),
                entries: Vec::new(),
                error: Some(err),
            },
        }
    }

    fn stat(&self, uri: &AwamUri) -> StatResult {
        if uri.path.is_empty() {
            return StatResult {
                entry: Some(ProviderEntry {
                    name: "rclone".to_string(),
                    uri: uri.to_string_uri(),
                    is_dir: true,
                    is_file: false,
                    size: 0,
                    modified_time: 0,
                    local_path: None,
                    extra: HashMap::new(),
                }),
                error: None,
            };
        }

        let (parent_path, name) = match uri.path.rsplit_once('/') {
            Some((parent, name)) => (parent.to_string(), name.to_string()),
            None => (String::new(), uri.path.clone()),
        };
        let parent = AwamUri {
            backend: "rclone".to_string(),
            path: parent_path,
            query: HashMap::new(),
        };
        if let Some(entry) = self
            .list(&parent)
            .entries
            .into_iter()
            .find(|entry| entry.name == name)
        {
            return StatResult {
                entry: Some(entry),
                error: None,
            };
        }

        let mut extra = HashMap::new();
        extra.insert("virtual".to_string(), Value::Bool(true));
        StatResult {
            entry: Some(ProviderEntry {
                name,
                uri: uri.to_string_uri(),
                is_dir: true,
                is_file: false,
                size: 0,
                modified_time: 0,
                local_path: None,
                extra,
            }),
            error: None,
        }
    }

    fn resolve(&self, uri: &AwamUri) -> ResolveResult {
        match Self::remote_spec(uri) {
            Some(spec) => {
                let mut action_args = HashMap::new();
                action_args.insert("rclone_spec".to_string(), Value::String(spec));
                ResolveResult {
                    uri: uri.to_string_uri(),
                    local_path: None,
                    action: Some("rclone_spec".to_string()),
                    action_args,
                    error: None,
                }
            }
            None => ResolveResult {
                uri: uri.to_string_uri(),
                local_path: None,
                action: None,
                action_args: HashMap::new(),
                error: Some("rclone root has no remote spec".to_string()),
            },
        }
    }
}

fn parse_lsjson(stdout: &str, parent: &AwamUri) -> Result<Vec<ProviderEntry>, String> {
    let value: Value =
        serde_json::from_str(stdout).map_err(|err| format!("Invalid rclone lsjson: {err}"))?;
    let items = value
        .as_array()
        .ok_or_else(|| "rclone lsjson did not return an array".to_string())?;
    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let name = item
            .get("Name")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let is_dir = item
            .get("IsDir")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let size = item.get("Size").and_then(|value| value.as_u64()).unwrap_or(0);
        let mut extra = HashMap::new();
        extra.insert("virtual".to_string(), Value::Bool(true));
        entries.push(ProviderEntry {
            uri: parent.child(&name).to_string_uri(),
            name,
            is_dir,
            is_file: !is_dir,
            size: if is_dir { 0 } else { size },
            modified_time: 0,
            local_path: None,
            extra,
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockRclone {
        remotes: Mutex<Vec<String>>,
        lsjson: Mutex<String>,
        last_args: Mutex<Vec<String>>,
    }

    impl RcloneRunner for MockRclone {
        fn run(&self, args: &[&str], _timeout: Duration) -> Result<CommandOutput, String> {
            *self.last_args.lock().unwrap() =
                args.iter().map(|s| (*s).to_string()).collect();
            if args.first() == Some(&"listremotes") {
                let stdout = self
                    .remotes
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|name| format!("{name}:"))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Ok(CommandOutput {
                    status_code: 0,
                    stdout,
                    stderr: String::new(),
                });
            }
            if args.first() == Some(&"lsjson") {
                return Ok(CommandOutput {
                    status_code: 0,
                    stdout: self.lsjson.lock().unwrap().clone(),
                    stderr: String::new(),
                });
            }
            Err(format!("unexpected args: {args:?}"))
        }
    }

    #[test]
    fn lists_remotes_and_lsjson_without_mount() {
        let runner = Arc::new(MockRclone {
            remotes: Mutex::new(vec!["mysftp".to_string(), "b2".to_string()]),
            lsjson: Mutex::new(
                r#"[{"Name":"var","IsDir":true,"Size":0},{"Name":"readme.txt","IsDir":false,"Size":12}]"#
                    .to_string(),
            ),
            last_args: Mutex::new(Vec::new()),
        });
        let provider = RcloneProvider::new(runner.clone());
        assert!(provider.health().healthy);

        let root = provider.list(&AwamUri {
            backend: "rclone".to_string(),
            path: String::new(),
            query: HashMap::new(),
        });
        assert_eq!(root.entries.len(), 2);
        assert!(root
            .entries
            .iter()
            .all(|entry| entry.extra.get("virtual") == Some(&Value::Bool(true))));

        let listing = provider.list(&AwamUri {
            backend: "rclone".to_string(),
            path: "mysftp".to_string(),
            query: HashMap::new(),
        });
        assert!(listing.error.is_none());
        assert_eq!(listing.entries.len(), 2);
        assert!(listing
            .entries
            .iter()
            .any(|entry| entry.name == "readme.txt" && entry.is_file));

        let args = runner.last_args.lock().unwrap().clone();
        assert_eq!(args.first().as_deref(), Some(&"lsjson".to_string()));
        assert!(args.iter().any(|a| a == "--max-depth"));
        assert!(args.iter().any(|a| a == "--contimeout"));
        assert!(args.iter().any(|a| a == "mysftp:"));

        let resolved = provider.resolve(&AwamUri {
            backend: "rclone".to_string(),
            path: "mysftp/var".to_string(),
            query: HashMap::new(),
        });
        assert_eq!(resolved.action.as_deref(), Some("rclone_spec"));
        assert_eq!(
            resolved.action_args.get("rclone_spec"),
            Some(&Value::String("mysftp:var".to_string()))
        );
    }

    #[test]
    fn remote_list_surfaces_timeout_error() {
        struct TimeoutRunner;
        impl RcloneRunner for TimeoutRunner {
            fn run(&self, _args: &[&str], timeout: Duration) -> Result<CommandOutput, String> {
                Err(format!(
                    "rclone timed out after {}s (remote may be unreachable or waiting for credentials)",
                    timeout.as_secs().max(1)
                ))
            }
        }
        let provider = RcloneProvider::new(Arc::new(TimeoutRunner));
        let listing = provider.list(&AwamUri {
            backend: "rclone".to_string(),
            path: "dy-c1".to_string(),
            query: HashMap::new(),
        });
        assert!(listing.entries.is_empty());
        let err = listing.error.unwrap_or_default();
        assert!(err.contains("timed out"), "{err}");
    }

    #[test]
    fn join_bytes_timeout_returns_empty() {
        let handle = thread::spawn(|| {
            thread::sleep(Duration::from_secs(2));
            b"late".to_vec()
        });
        let buf = join_bytes_with_timeout(handle, Duration::from_millis(50));
        assert!(buf.is_empty());
    }

    #[test]
    fn missing_rclone_message_is_user_facing() {
        let msg = rclone_missing_message(None);
        assert!(msg.contains("rclone"), "{msg}");
        assert!(!msg.contains("Day-1"), "{msg}");
        let configured = rclone_missing_message(Some(r"Z:\missing\rclone.exe"));
        assert!(configured.contains("Z:\\missing\\rclone.exe"), "{configured}");
    }

    #[test]
    fn path_candidates_include_platform_exe() {
        let names = rclone_path_candidates();
        assert!(names.contains(&"rclone"));
        #[cfg(windows)]
        assert!(names.contains(&"rclone.exe"));
    }
}
