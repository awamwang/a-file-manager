// SPDX-License-Identifier: GPL-3.0-or-later
//! Recycle Bin list / restore via Shell.Application (Windows only).
//!
//! macOS has no equivalent here: `trash 5.2` deliberately excludes macOS from
//! `os_limited` (its list/restore/purge APIs are Windows + freedesktop only),
//! and Finder automation would require TCC automation permission. The platform
//! capability bit `trashListRestore` stays false, so the UI must hide these
//! commands and offer `open_recycle_bin_system()` (`~/.Trash` in Finder).

// 7.24: the payload DTO is neutral and lives in `platform/`; this module is
// the Windows Shell.Application backend behind `TrashProvider`.
pub use crate::platform::RecycleBinItem;
#[cfg(any(windows, test))]
use std::fs;
#[cfg(windows)]
use std::path::Path;
#[cfg(any(windows, test))]
use std::path::PathBuf;
#[cfg(windows)]
use std::process::Command;

/// List Recycle Bin entries (name + original path).
pub fn list_recycle_bin() -> Result<Vec<RecycleBinItem>, String> {
    #[cfg(not(windows))]
    {
        // `trash::os_limited` excludes macOS; no list/restore API to reuse.
        return Err(
            "Recycle Bin listing is unavailable on macOS; open ~/.Trash in Finder instead"
                .to_string(),
        );
    }

    #[cfg(windows)]
    {
        let script = r#"
$ErrorActionPreference = 'Stop'
$shell = New-Object -ComObject Shell.Application
$bin = $shell.NameSpace(10)
if (-not $bin) { '[]'; exit 0 }
$items = @()
foreach ($item in $bin.Items()) {
  $origDir = $item.ExtendedProperty('System.Recycle.DeletedFrom')
  if (-not $origDir) { $origDir = $bin.GetDetailsOf($item, 1) }
  $name = $item.Name
  $orig = if ($origDir) { Join-Path $origDir $name } else { $name }
  $items += [pscustomobject]@{
    name = $name
    originalPath = $orig
    recyclePath = $item.Path
  }
}
$items | ConvertTo-Json -Compress -Depth 3
"#;
        let output = run_powershell(script)?;
        parse_recycle_json(&output)
    }
}

#[cfg(any(windows, test))]
fn parse_recycle_json(raw: &str) -> Result<Vec<RecycleBinItem>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return Ok(Vec::new());
    }
    // Single object vs array.
    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("Failed to parse Recycle Bin listing: {err}"))?;
    let list = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(_) => vec![value],
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for item in list {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let original_path = item
            .get("original_path")
            .or_else(|| item.get("originalPath"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let recycle_path = item
            .get("recycle_path")
            .or_else(|| item.get("recyclePath"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() || recycle_path.is_empty() {
            continue;
        }
        out.push(RecycleBinItem {
            name,
            original_path,
            recycle_path,
        });
    }
    Ok(out)
}

#[cfg(windows)]
fn run_powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|err| format!("Failed to start PowerShell: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Recycle Bin PowerShell failed: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(any(windows, test))]
fn write_wanted_paths_file(original_paths: &[String]) -> Result<PathBuf, String> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "afm-restore-wanted-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    let mut body = String::new();
    for item in original_paths {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        body.push_str(&trimmed.replace('/', "\\"));
        body.push('\n');
    }
    fs::write(&path, body).map_err(|err| format!("Failed to write restore path list: {err}"))?;
    Ok(path)
}

#[cfg(any(windows, test))]
fn parse_restored_paths(raw: &str) -> Result<Vec<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "null" || trimmed == "[]" {
        return Ok(Vec::new());
    }
    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("Failed to parse restore result: {err}"))?;
    match value {
        serde_json::Value::Array(items) => Ok(items
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .collect()),
        serde_json::Value::String(one) => {
            if one.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![one])
            }
        }
        _ => Ok(Vec::new()),
    }
}

/// Restore Recycle Bin items whose original path matches any of `original_paths`.
/// No matching items means they are already gone from the bin (restored or
/// emptied elsewhere): return an empty list instead of an error so callers can
/// treat it as a silent no-op (e.g. Ctrl+Z undo).
///
/// 在一次 PowerShell 会话中完成「列举 + 匹配 + 还原」，避免按文件反复拉起
/// 进程并全量扫描回收站（多文件撤销时会严重阻塞）。
pub fn restore_original_paths(original_paths: &[String]) -> Result<Vec<String>, String> {
    #[cfg(not(windows))]
    {
        let _ = original_paths;
        return Err(
            "Recycle Bin restore is unavailable on macOS; use Finder's Trash instead".to_string(),
        );
    }
    #[cfg(windows)]
    {
        if original_paths.is_empty() {
            return Ok(Vec::new());
        }
        let wanted_file = write_wanted_paths_file(original_paths)?;
        let result = restore_original_paths_with_list(&wanted_file);
        let _ = fs::remove_file(&wanted_file);
        result
    }
}

#[cfg(windows)]
fn restore_original_paths_with_list(wanted_file: &Path) -> Result<Vec<String>, String> {
    let escaped = wanted_file.to_string_lossy().replace('\'', "''");
    let script = restore_wanted_script(&escaped);
    let output = run_powershell(&script)?;
    parse_restored_paths(&output)
}

/// The one-pass PowerShell batch-restore script (list + match + restore).
/// Extracted from `restore_original_paths_with_list` so tests can pin its
/// behavior-critical fragments without spawning a real PowerShell/Recycle Bin.
#[cfg(windows)]
fn restore_wanted_script(escaped: &str) -> String {
    format!(
        r#"
$ErrorActionPreference = 'Stop'
function Normalize-PathKey([string]$p) {{
  if (-not $p) {{ return '' }}
  $n = $p.Trim().Replace('/', '\').TrimEnd('\')
  $parent = Split-Path -Parent $n
  $leaf = Split-Path -Leaf $n
  if ($parent -and $leaf -and (Test-Path -LiteralPath $parent)) {{
    try {{
      $fso = New-Object -ComObject Scripting.FileSystemObject
      $n = Join-Path $fso.GetFolder($parent).Path $leaf
    }} catch {{}}
  }}
  return $n.ToLowerInvariant()
}}
$wantedSet = New-Object 'System.Collections.Generic.HashSet[string]'
Get-Content -LiteralPath '{escaped}' -Encoding UTF8 | ForEach-Object {{
  $key = Normalize-PathKey $_
  if ($key) {{ [void]$wantedSet.Add($key) }}
}}
if ($wantedSet.Count -eq 0) {{ '[]'; exit 0 }}
$shell = New-Object -ComObject Shell.Application
$bin = $shell.NameSpace(10)
if (-not $bin) {{ '[]'; exit 0 }}
$restored = New-Object System.Collections.Generic.List[string]
foreach ($item in $bin.Items()) {{
  if ($wantedSet.Count -eq 0) {{ break }}
  $origDir = $item.ExtendedProperty('System.Recycle.DeletedFrom')
  if (-not $origDir) {{ $origDir = $bin.GetDetailsOf($item, 1) }}
  $name = $item.Name
  $orig = if ($origDir) {{ Join-Path $origDir $name }} else {{ $name }}
  $key = Normalize-PathKey $orig
  if (-not $wantedSet.Contains($key)) {{ continue }}
  $verb = $null
  foreach ($v in $item.Verbs()) {{
    $label = ($v.Name -replace '&','').Trim()
    if ($label -eq 'Restore' -or $label -eq '还原') {{ $verb = $v; break }}
  }}
  if (-not $verb) {{ continue }}
  # DEL-007: the original parent chain may be gone (Explorer recreates it on
  # restore too) — rebuild it before DoIt() so the restore is not lost.
  if ($origDir -and -not (Test-Path -LiteralPath $origDir)) {{
    try {{ New-Item -ItemType Directory -Path $origDir -Force | Out-Null }} catch {{}}
  }}
  $verb.DoIt()
  $waited = 0
  while (-not (Test-Path -LiteralPath $orig) -and $waited -lt 40) {{ Start-Sleep -Milliseconds 250; $waited++ }}
  if (-not (Test-Path -LiteralPath $orig)) {{ continue }}
  [void]$restored.Add($orig)
  [void]$wantedSet.Remove($key)
}}
if ($restored.Count -eq 0) {{ '[]' }} elseif ($restored.Count -eq 1) {{ ConvertTo-Json -Compress -InputObject @($restored[0]) }} else {{ $restored | ConvertTo-Json -Compress }}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_array_and_object() {
        let one = r#"{"name":"a.txt","originalPath":"C:\\a.txt","recyclePath":"::{trash}\\a"}"#;
        let items = parse_recycle_json(one).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "a.txt");

        let many = r#"[{"name":"a.txt","originalPath":"C:\\a.txt","recyclePath":"x"},{"name":"b","originalPath":"D:\\b","recyclePath":"y"}]"#;
        assert_eq!(parse_recycle_json(many).unwrap().len(), 2);
    }

    #[test]
    fn parse_restored_paths_shapes() {
        assert!(parse_restored_paths("[]").unwrap().is_empty());
        assert_eq!(
            parse_restored_paths(r#""C:\\a.txt""#).unwrap(),
            vec![r"C:\a.txt".to_string()]
        );
        assert_eq!(
            parse_restored_paths(r#"["C:\\a.txt","D:\\b.txt"]"#)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn write_wanted_paths_file_roundtrip() {
        let file = write_wanted_paths_file(&[
            r"C:\Temp\a.txt".into(),
            "D:/Temp/b.txt".into(),
            "  ".into(),
        ])
        .unwrap();
        let body = fs::read_to_string(&file).unwrap();
        let _ = fs::remove_file(&file);
        assert!(body.contains(r"C:\Temp\a.txt"));
        assert!(body.contains(r"D:\Temp\b.txt"));
        assert!(!body.contains("D:/Temp/b.txt"));
    }

    // DEL-007 (已拍板 2026-11-22): when the original parent chain is gone the
    // restore must recreate it (Explorer parity) instead of losing the item.
    // The batch script is the only place restore happens, so pin the recreation
    // fragment + the embedded wanted-list path (escaping round-trip).
    #[cfg(windows)]
    #[test]
    fn restore_script_recreates_missing_parent_chain() {
        let wanted = std::env::temp_dir().join("afm-del007-wanted'.txt");
        let escaped = wanted.to_string_lossy().replace('\'', "''");
        let script = restore_wanted_script(&escaped);
        assert!(
            script.contains("New-Item -ItemType Directory -Path $origDir -Force"),
            "DEL-007: script must recreate the vanished original parent chain"
        );
        assert!(
            script.contains(&escaped),
            "wanted-list path must be embedded with quotes escaped"
        );
        assert!(
            !script.contains("''''"),
            "double-escaping would break the LiteralPath"
        );
    }

    /// DEL-007 real-fixture proof (machine-state dependent, `#[ignore]`d per
    /// ADR 0025): delete a file to the *real* Recycle Bin, remove its original
    /// parent chain, then restore — the restore must recreate the vanished
    /// parent directory and bring the content back intact (Explorer parity).
    /// Run: `cargo test --lib real_recycle -- --ignored --nocapture`.
    #[cfg(windows)]
    #[ignore]
    #[test]
    fn real_recycle_restore_recreates_deleted_parent_dir() {
        use crate::file_ops::{delete_paths, display_path};
        // Long-form base dir: `std::env::temp_dir()` may return an 8.3 short
        // path (C:\Users\ADMINI~1\…) while the bin records the DeletedFrom in
        // long form; with the parent chain gone, Normalize-PathKey can no
        // longer expand the short form, so the wanted key would never match.
        let short = std::env::temp_dir().join(format!(
            "afm-del007-real-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        fs::create_dir_all(&short).unwrap();
        let long = fs::canonicalize(&short).unwrap();
        let dir = PathBuf::from(
            long.to_string_lossy()
                .trim_start_matches(r"\\?\")
                .to_string(),
        );
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        let file = sub.join("victim.txt");
        fs::write(&file, b"del-007 real fixture").unwrap();
        let original = display_path(&file);

        delete_paths(&[original.clone()]).expect("delete to real Recycle Bin");
        assert!(!file.exists());
        fs::remove_dir_all(&sub).expect("remove the original parent chain");
        assert!(!sub.exists());

        let restored = restore_original_paths(&[original.clone()])
            .expect("restore from real Recycle Bin");
        assert_eq!(
            restored,
            vec![original.clone()],
            "the victim must be reported restored to its original path"
        );
        assert!(sub.exists(), "DEL-007: vanished parent dir must be recreated");
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "del-007 real fixture",
            "restored content must be intact"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
