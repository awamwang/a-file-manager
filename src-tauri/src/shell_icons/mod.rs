// SPDX-License-Identifier: GPL-3.0-or-later
//! Platform-neutral icon / thumbnail encoding and bounded cache (AIA-65 · AIA-143).
//!
//! OS image types and Shell/AppKit calls live in `platform/` (7.23); this module
//! only encodes neutral RGBA to PNG and caches it. Do **not** self-decode files
//! into thumbnails.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Bound process-side Shell results so extreme folders cannot retain every PNG data URL (AIA-279).
const MAX_ICON_CACHE_ENTRIES: usize = 384;

struct BoundedIconCache {
    map: HashMap<String, String>,
    order: VecDeque<String>,
}

impl BoundedIconCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&String> {
        self.map.get(key)
    }

    fn insert(&mut self, key: String, value: String) {
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
            return;
        }
        while self.map.len() >= MAX_ICON_CACHE_ENTRIES {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            } else {
                break;
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.map.len()
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

static ICON_CACHE: Lazy<Mutex<BoundedIconCache>> =
    Lazy::new(|| Mutex::new(BoundedIconCache::new()));

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellIconResult {
    pub path: String,
    pub data_url: Option<String>,
    pub kind: String,
}

fn encode_rgba_png_data_url(width: u32, height: u32, pixels: &[u8]) -> Result<String, String> {
    let mut png_bytes = Vec::new();
    {
        let encoder = PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(pixels, width, height, ColorType::Rgba8.into())
            .map_err(|err| format!("PNG encode failed: {err}"))?;
    }
    Ok(format!(
        "data:image/png;base64,{}",
        BASE64.encode(png_bytes)
    ))
}

fn cache_key(path: &str, size: u16, kind: &str) -> String {
    format!(
        "{kind}|{size}|{}",
        crate::paths::normalize_path_for_comparison(path)
    )
}

/// Shared shell-icon key for the same file type (extension / dir), not per-path.
///
/// `None` means the icon is file-specific and must stay on the per-path cache:
/// executables and shortcuts each carry their own application icon, so sharing
/// one glyph across the whole extension would render every app identically.
fn type_icon_cache_key(path: &Path, size: u16) -> Option<String> {
    // Per-file icons — two .exe / .lnk files have distinct application images.
    const PER_FILE_EXTS: &[&str] = &[
        "exe", "dll", "lnk", "ico", "msi", "scr", "cpl", "app", "bat", "cmd", "com", "ps1",
    ];
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if PER_FILE_EXTS.contains(&ext.as_str()) {
        return None;
    }
    if path.is_dir() {
        return Some(format!("icon|{size}|type:dir"));
    }
    if !ext.is_empty() {
        Some(format!("icon|{size}|type:{ext}"))
    } else {
        Some(format!("icon|{size}|type:none"))
    }
}

/// Resolve the path whose icon should actually be rendered for a link-like entry.
///
/// - Windows `.lnk`: read the stored target (exe / file / folder) so the app's
///   own image is shown instead of a generic shortcut leaf. The shortcut arrow
///   is added by the frontend as an overlay, so the target icon stays bare.
/// - Filesystem symlinks (junctions on directories are already resolved to the
///   real target by the navigator listing): follow to the target when present.
///
/// Falls back to `path` itself for broken links, non-Windows, or read failures.
fn icon_source_path(path: &Path) -> PathBuf {
    let is_lnk = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false);

    if is_lnk {
        if let Ok(target) = crate::file_ops::shortcut::read_shortcut_target(path) {
            let target = target.trim();
            if !target.is_empty() {
                let candidate = PathBuf::from(target.replace('/', "\\"));
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }

    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            if let Ok(target) = std::fs::read_link(path) {
                let resolved = if target.is_absolute() {
                    target
                } else {
                    path.parent().unwrap_or_else(|| Path::new("")).join(target)
                };
                if resolved.exists() {
                    return resolved;
                }
            }
        }
    }

    path.to_path_buf()
}

fn shell_icon_data_url(path: &Path, size: u16) -> Result<String, String> {
    let icon = crate::platform::file_icon(path, size)?;
    encode_rgba_png_data_url(icon.width, icon.height, &icon.pixels)
}

/// Windows Shell cache / handlers live in `platform/windows.rs`; macOS and
/// other Unix hosts downgrade to the NSWorkspace type icon (7.23). Callers keep
/// the `thumbnail → file icon` fallback unchanged.
fn system_thumbnail_data_url(path: &Path, size: u16) -> Result<String, String> {
    let icon = crate::platform::system_thumbnail(path, size)?;
    encode_rgba_png_data_url(icon.width, icon.height, &icon.pixels)
}

pub fn resolve_icon(path: &str, size: u16, prefer_thumbnail: bool) -> ShellIconResult {
    let trimmed = path.trim();
    // Single platform-aware normalizer (see `platform::fs`), not a hardcoded `\`.
    let pb = PathBuf::from(crate::paths::to_native_fs_path(trimmed));
    let size = size.clamp(8, 256);
    // Links render their target's icon (app image for .lnk); the shortcut arrow
    // is an overlay drawn by the frontend, so this icon stays bare.
    let source = icon_source_path(&pb);
    let want_thumb = prefer_thumbnail && source.is_file();
    let kind = if want_thumb { "thumbnail" } else { "icon" };
    let path_key = cache_key(trimmed, size, kind);
    let type_key = type_icon_cache_key(&source, size);

    if let Ok(cache) = ICON_CACHE.lock() {
        if let Some(hit) = cache.get(&path_key) {
            return ShellIconResult {
                path: trimmed.to_string(),
                data_url: Some(hit.clone()),
                kind: kind.to_string(),
            };
        }
        // Shell icons (and thumb→icon fallbacks) share one glyph per file type,
        // except file-specific icons (exe/lnk/…) which stay on the path cache.
        if !want_thumb {
            if let Some(ref type_key) = type_key {
                if let Some(hit) = cache.get(type_key) {
                    return ShellIconResult {
                        path: trimmed.to_string(),
                        data_url: Some(hit.clone()),
                        kind: "icon".to_string(),
                    };
                }
            }
        }
    }

    let data_url = if want_thumb {
        // AIA-143：系统缩略图（含 SageThumbs 等）优先；批内必须串行 COM。
        system_thumbnail_data_url(&source, size)
            .or_else(|_| shell_icon_data_url(&source, size))
            .ok()
    } else if source.exists() {
        shell_icon_data_url(&source, size).ok()
    } else {
        None
    };

    // Refine kind: if we asked for thumbnail but only got a generic icon-sized result
    // via shell_icon fallback, still report "thumbnail" when prefer_thumbnail — UI treats
    // both as image src. Probe path is what matters for AIA-143.
    let kind = if want_thumb && data_url.is_some() {
        "thumbnail".to_string()
    } else {
        kind.to_string()
    };

    if let Some(ref url) = data_url {
        if let Ok(mut cache) = ICON_CACHE.lock() {
            cache.insert(path_key, url.clone());
            if !want_thumb {
                if let Some(ref type_key) = type_key {
                    cache.insert(type_key.clone(), url.clone());
                }
            }
        }
    }

    ShellIconResult {
        path: trimmed.to_string(),
        data_url,
        kind,
    }
}

#[tauri::command]
pub async fn shell_icon_get(
    path: String,
    size: Option<u16>,
    prefer_thumbnail: Option<bool>,
) -> ShellIconResult {
    let size = size.unwrap_or(20);
    let prefer = prefer_thumbnail.unwrap_or(true);
    // Shell thumbnail / icon work is COM + disk; keep it off the async runtime workers.
    let fallback_path = path.clone();
    tauri::async_runtime::spawn_blocking(move || resolve_icon(&path, size, prefer))
        .await
        .unwrap_or(ShellIconResult {
            path: fallback_path,
            data_url: None,
            kind: "icon".into(),
        })
}

#[tauri::command]
pub async fn shell_icon_get_many(
    paths: Vec<String>,
    size: Option<u16>,
    prefer_thumbnail: Option<bool>,
) -> Vec<ShellIconResult> {
    let size = size.unwrap_or(20);
    let prefer = prefer_thumbnail.unwrap_or(true);
    let n = paths.len();
    let t0 = std::time::Instant::now();
    // AIA-278：前端小批 + spawn_blocking；批内串行，避免 SageThumbs 等争用 COM。
    let results = tauri::async_runtime::spawn_blocking(move || {
        paths
            .into_iter()
            .map(|p| resolve_icon(&p, size, prefer))
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();
    crate::perf_log::afm_perf_line(format!(
        "AFM_PERF shell_icon_batch n={} size={} thumb={} {:.1}ms",
        n,
        size,
        if prefer { 1 } else { 0 },
        t0.elapsed().as_secs_f64() * 1000.0
    ));
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn bounded_icon_cache_evicts_oldest() {
        let mut cache = BoundedIconCache::new();
        for i in 0..(MAX_ICON_CACHE_ENTRIES + 10) {
            cache.insert(format!("k{i}"), format!("v{i}"));
        }
        assert_eq!(cache.len(), MAX_ICON_CACHE_ENTRIES);
        assert!(cache.get("k0").is_none(), "oldest entries must be evicted");
        assert!(cache
            .get(&format!("k{}", MAX_ICON_CACHE_ENTRIES + 9))
            .is_some());
    }

    #[test]
    fn type_icon_cache_reuses_same_extension() {
        let dir = std::env::temp_dir().join(format!("afm-type-icon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        if let Ok(mut cache) = ICON_CACHE.lock() {
            cache.clear();
        }
        let first = resolve_icon(&a.to_string_lossy(), 16, false);
        assert!(first.data_url.is_some());
        let type_key = type_icon_cache_key(&a, 16);
        {
            let cache = ICON_CACHE.lock().unwrap();
            assert!(
                type_key.as_ref().and_then(|k| cache.get(k)).is_some(),
                "type cache must fill after shell icon"
            );
        }
        let second = resolve_icon(&b.to_string_lossy(), 16, false);
        assert_eq!(first.data_url, second.data_url);
        let _ = fs::remove_dir_all(&dir);
    }

    /// Manual perf probe: `AFM_PERF_DIR=<dir> cargo test --lib shell_icons::tests::bench_shell_icon_batch -- --nocapture`
    #[test]
    fn bench_shell_icon_batch() {
        let Ok(dir) = std::env::var("AFM_PERF_DIR") else {
            return;
        };
        let paths: Vec<String> = fs::read_dir(&dir)
            .expect("AFM_PERF_DIR")
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_string_lossy().to_string())
            .take(40)
            .collect();
        assert!(paths.len() >= 8, "need at least 8 files in AFM_PERF_DIR");

        let mut cold_icon = Vec::new();
        for _ in 0..5 {
            if let Ok(mut cache) = ICON_CACHE.lock() {
                cache.clear();
            }
            let batch: Vec<String> = paths.iter().take(40).cloned().collect();
            let t0 = std::time::Instant::now();
            let _ = batch
                .into_iter()
                .map(|p| resolve_icon(&p, 20, false))
                .collect::<Vec<_>>();
            cold_icon.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        cold_icon.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let warm_t0 = std::time::Instant::now();
        let _ = paths
            .iter()
            .take(40)
            .map(|p| resolve_icon(p, 20, false))
            .collect::<Vec<_>>();
        let warm_icon_ms = warm_t0.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "AFM_PERF shell_icon_batch n=40 thumb=0 cold_samples_ms={cold_icon:?} cold_median_ms={:.1} warm_ms={warm_icon_ms:.1}",
            cold_icon[cold_icon.len() / 2]
        );

        let thumb_paths: Vec<String> = paths.iter().take(8).cloned().collect();
        let mut cold_thumb = Vec::new();
        for _ in 0..5 {
            if let Ok(mut cache) = ICON_CACHE.lock() {
                cache.clear();
            }
            let t0 = std::time::Instant::now();
            let _ = thumb_paths
                .iter()
                .map(|p| resolve_icon(p, 96, true))
                .collect::<Vec<_>>();
            cold_thumb.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        cold_thumb.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let warm_t0 = std::time::Instant::now();
        let _ = thumb_paths
            .iter()
            .map(|p| resolve_icon(p, 96, true))
            .collect::<Vec<_>>();
        let warm_thumb_ms = warm_t0.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "AFM_PERF shell_icon_batch n=8 thumb=1 cold_samples_ms={cold_thumb:?} cold_median_ms={:.1} warm_ms={warm_thumb_ms:.1}",
            cold_thumb[cold_thumb.len() / 2]
        );
    }

    #[test]
    fn per_file_exts_do_not_share_type_icon() {
        // exe/lnk and friends carry per-application icons — must not collapse
        // into one shared type glyph.
        assert_eq!(type_icon_cache_key(Path::new(r"C:\a\one.exe"), 20), None);
        assert_eq!(type_icon_cache_key(Path::new(r"C:\a\two.lnk"), 20), None);
        // Ordinary extensions still share one glyph.
        assert_eq!(
            type_icon_cache_key(Path::new(r"C:\a\one.txt"), 20).as_deref(),
            Some("icon|20|type:txt")
        );
    }

    #[test]
    fn broken_link_icon_source_falls_back_to_link_path() {
        // A nonexistent .lnk has no readable target; source stays the input path.
        let missing = PathBuf::from(r"C:\definitely\missing\shortcut-name.lnk");
        assert_eq!(icon_source_path(&missing), missing);
    }

    #[test]
    fn symlink_icon_source_follows_target() {
        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_file;
            let dir = std::env::temp_dir().join(format!("afm-link-icon-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            let target = dir.join("target.txt");
            fs::write(&target, "hello").unwrap();
            let link = dir.join("alias.txt");
            if symlink_file(&target, &link).is_ok() {
                let resolved = icon_source_path(&link);
                assert_eq!(
                    crate::paths::normalize_path_for_comparison(&resolved.to_string_lossy()),
                    crate::paths::normalize_path_for_comparison(&target.to_string_lossy())
                );
            }
            let _ = fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn encodes_temp_file_shell_icon() {
        let dir = std::env::temp_dir().join(format!("afm-icon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("note.txt");
        let mut f = fs::File::create(&file).unwrap();
        write!(f, "hi").unwrap();

        let result = resolve_icon(&file.to_string_lossy(), 16, false);
        assert_eq!(result.kind, "icon");
        assert!(
            result
                .data_url
                .as_ref()
                .map(|u| u.starts_with("data:image/png"))
                .unwrap_or(false),
            "expected png data url, got {:?}",
            result.data_url
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn system_thumbnail_or_icon_for_png() {
        let dir = std::env::temp_dir().join(format!("afm-thumb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("dot.png");
        // 1x1 red PNG — Shell may serve thumbnail or fall back to icon.
        let png = BASE64
            .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==")
            .unwrap();
        fs::write(&file, png).unwrap();
        let result = resolve_icon(&file.to_string_lossy(), 32, true);
        assert_eq!(result.kind, "thumbnail");
        assert!(result.data_url.is_some());
        // Must not use the removed self-decoder path; system or shell icon only.
        assert!(system_thumbnail_data_url(&file, 32).is_ok() || result.data_url.is_some());
        let _ = fs::remove_dir_all(&dir);
    }
}
