// SPDX-License-Identifier: GPL-3.0-or-later

use super::provider::Provider;
use super::types::{
    AwamUri, BackendInfo, DirListing, ProviderCapability, ProviderEntry, ResolveResult, StatResult,
};
use crate::win_process;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::process::Stdio;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const CAPABILITIES: &[ProviderCapability] = &[
    ProviderCapability::List,
    ProviderCapability::Stat,
    ProviderCapability::Resolve,
];

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait RcloneRunner: Send + Sync {
    fn run(&self, args: &[&str], timeout: Duration) -> Result<CommandOutput, String>;
}

/// rclone CLI runner (no mount / no VFS). Honors `timeout` by killing the child (AIA-182).
#[derive(Debug, Default)]
pub struct BlockingRcloneRunner {
    pub exe: String,
}

impl BlockingRcloneRunner {
    pub fn new() -> Self {
        Self {
            exe: std::env::var("AWAM_RCLONE_EXE")
                .or_else(|_| std::env::var("RCLONE_EXE"))
                .unwrap_or_else(|_| "rclone".to_string()),
        }
    }
}

impl RcloneRunner for BlockingRcloneRunner {
    fn run(&self, args: &[&str], timeout: Duration) -> Result<CommandOutput, String> {
        // rclone is a console binary; CREATE_NO_WINDOW prevents flash on GUI builds
        // (provider_preload / health / listremotes at app start and on Providers refresh).
        let mut child = win_process::command(&self.exe)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    "rclone not found on PATH".to_string()
                } else {
                    format!("Failed to run rclone: {err}")
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
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "rclone timed out after {}s",
                        timeout.as_secs().max(1)
                    ));
                }
                Ok(None) => thread::sleep(Duration::from_millis(25)),
                Err(err) => return Err(format!("Failed to wait for rclone: {err}")),
            }
        };

        let stdout = String::from_utf8_lossy(&stdout_thr.join().unwrap_or_default()).to_string();
        let stderr = String::from_utf8_lossy(&stderr_thr.join().unwrap_or_default()).to_string();
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
            .run(&["listremotes"], Duration::from_secs(15))?;
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
}

impl Provider for RcloneProvider {
    fn backend_id(&self) -> &str {
        "rclone"
    }
    fn name(&self) -> &str {
        "rclone"
    }
    fn description(&self) -> &str {
        "Cloud / SFTP / WebDAV via rclone CLI (no mount)"
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
                    "No remotes configured (rclone config)".to_string()
                } else {
                    format!("{} remote(s)", remotes.len())
                }),
                interface_only: false,
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
                    error: Some("No rclone remotes. Run: rclone config".to_string()),
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

        match self
            .runner
            .run(&["lsjson", &spec], Duration::from_secs(60))
        {
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
    }

    impl RcloneRunner for MockRclone {
        fn run(&self, args: &[&str], _timeout: Duration) -> Result<CommandOutput, String> {
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
        });
        let provider = RcloneProvider::new(runner);
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
}
