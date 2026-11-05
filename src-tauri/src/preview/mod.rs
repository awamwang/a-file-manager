// SPDX-License-Identifier: GPL-3.0-or-later
//! Experimental image + plain-text preview for Navigator.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::Path;

const MAX_TEXT_BYTES: u64 = 256 * 1024;
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPayload {
    pub kind: String,
    pub path: String,
    pub text: Option<String>,
    pub data_url: Option<String>,
    pub truncated: bool,
    pub error: Option<String>,
}

fn extension_lower(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_image_ext(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico"
    )
}

fn is_text_ext(ext: &str) -> bool {
    matches!(
        ext,
        "txt"
            | "md"
            | "markdown"
            | "json"
            | "jsonc"
            | "csv"
            | "tsv"
            | "log"
            | "xml"
            | "html"
            | "htm"
            | "css"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "vue"
            | "rs"
            | "py"
            | "toml"
            | "yaml"
            | "yml"
            | "ini"
            | "cfg"
            | "conf"
            | "gitignore"
            | "env"
            | "svg"
    )
}

fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn unsupported(path: &str, truncated: bool, error: Option<String>) -> PreviewPayload {
    PreviewPayload {
        kind: "unsupported".into(),
        path: path.to_string(),
        text: None,
        data_url: None,
        truncated,
        error,
    }
}

#[tauri::command]
pub fn preview_load(path: String) -> PreviewPayload {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return unsupported("", false, Some("Path is empty".into()));
    }
    let pb = Path::new(trimmed);
    if !pb.is_file() {
        return unsupported(trimmed, false, Some("Not a file".into()));
    }
    let ext = extension_lower(pb);
    let meta = match fs::metadata(pb) {
        Ok(m) => m,
        Err(err) => {
            return unsupported(trimmed, false, Some(format!("Failed to read metadata: {err}")));
        }
    };

    if is_image_ext(&ext) {
        if meta.len() > MAX_IMAGE_BYTES {
            return unsupported(
                trimmed,
                true,
                Some(format!(
                    "Image too large for preview (max {} MB)",
                    MAX_IMAGE_BYTES / (1024 * 1024)
                )),
            );
        }
        return match fs::read(pb) {
            Ok(bytes) => PreviewPayload {
                kind: "image".into(),
                path: trimmed.to_string(),
                text: None,
                data_url: Some(format!(
                    "data:{};base64,{}",
                    mime_for_ext(&ext),
                    B64.encode(bytes)
                )),
                truncated: false,
                error: None,
            },
            Err(err) => unsupported(trimmed, false, Some(format!("Failed to read image: {err}"))),
        };
    }

    if is_text_ext(&ext) || ext.is_empty() {
        let mut file = match fs::File::open(pb) {
            Ok(f) => f,
            Err(err) => {
                return unsupported(trimmed, false, Some(format!("Failed to open file: {err}")));
            }
        };
        let truncated = meta.len() > MAX_TEXT_BYTES;
        let mut limited = file.by_ref().take(MAX_TEXT_BYTES);
        let mut buf = Vec::new();
        if let Err(err) = limited.read_to_end(&mut buf) {
            return unsupported(trimmed, false, Some(format!("Failed to read text: {err}")));
        }
        if buf.iter().filter(|b| **b == 0).count() > 8 {
            return unsupported(trimmed, false, Some("Binary file".into()));
        }
        let text = String::from_utf8_lossy(&buf).into_owned();
        return PreviewPayload {
            kind: "text".into(),
            path: trimmed.to_string(),
            text: Some(text),
            data_url: None,
            truncated,
            error: None,
        };
    }

    unsupported(trimmed, false, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_path_unsupported() {
        let p = preview_load(String::new());
        assert_eq!(p.kind, "unsupported");
    }

    #[test]
    fn text_preview_reads_file() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let path = std::env::temp_dir().join(format!("afm-preview-{stamp}.txt"));
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "hello preview").unwrap();
        let p = preview_load(path.to_string_lossy().into_owned());
        assert_eq!(p.kind, "text");
        assert!(p.text.unwrap().contains("hello preview"));
        let _ = fs::remove_file(path);
    }
}
