// SPDX-License-Identifier: GPL-3.0-or-later
//! Recycle Bin list / restore via Shell.Application (Windows).

use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecycleBinItem {
    pub name: String,
    pub original_path: String,
    /// Shell item path inside the bin (used for restore).
    pub recycle_path: String,
}

fn normalize_path(path: &str) -> String {
    path.trim()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

/// List Recycle Bin entries (name + original path).
pub fn list_recycle_bin() -> Result<Vec<RecycleBinItem>, String> {
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
            .get("originalPath")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let recycle_path = item
            .get("recyclePath")
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

/// Restore Recycle Bin items whose original path matches any of `original_paths`.
pub fn restore_original_paths(original_paths: &[String]) -> Result<Vec<String>, String> {
    if original_paths.is_empty() {
        return Ok(Vec::new());
    }
    let wanted: Vec<String> = original_paths.iter().map(|p| normalize_path(p)).collect();
    let items = list_recycle_bin()?;
    let mut restored = Vec::new();
    for item in items {
        let key = normalize_path(&item.original_path);
        if !wanted.iter().any(|w| w == &key) {
            continue;
        }
        restore_recycle_item(&item.recycle_path)?;
        restored.push(item.original_path);
    }
    if restored.is_empty() {
        return Err("No matching items found in Recycle Bin".to_string());
    }
    Ok(restored)
}

fn restore_recycle_item(recycle_path: &str) -> Result<(), String> {
    let escaped = recycle_path.replace('\'', "''");
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$shell = New-Object -ComObject Shell.Application
$bin = $shell.NameSpace(10)
$item = $bin.Items() | Where-Object {{ $_.Path -eq '{escaped}' }} | Select-Object -First 1
if (-not $item) {{ throw 'Recycle item not found' }}
$item.InvokeVerb('restore')
"#
    );
    let _ = run_powershell(&script)?;
    Ok(())
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
    fn normalize_compares_case() {
        assert_eq!(normalize_path(r"C:\Foo\"), r"c:\foo");
    }
}
