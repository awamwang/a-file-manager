// SPDX-License-Identifier: GPL-3.0-or-later
//! Launch folders/files in preset IDEs/Agents, and surface the *same* registry
//! to Project "open with" so the two flows share one source of truth.
//!
//! Auto-discover first; fall back to the Settings path when discovery fails.
//!
//! 7.20: the preset lists and the OS-specific spawn command moved to
//! `platform/`. This module owns the wire types, the settings override policy,
//! the "auto → configured → none" resolution order, *and* the OS-neutral
//! project behaviour (`supports_project` / `project_open_mode` / `folders_only`
//! / `project_args`) plus user-defined custom open tools persisted in settings.

use crate::platform::{self, LaunchTarget};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::AppHandle;

/// How an open tool should receive a Project workspace.
///
/// * `Workspace` — hand the workspace directory to the tool.
/// * `ContextFile` — open the workspace's `CONTEXT.md` (Qoder-style agents),
///   falling back to the directory when the file is missing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectOpenMode {
    #[default]
    Workspace,
    ContextFile,
}

impl ProjectOpenMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::ContextFile => "contextFile",
        }
    }
}

/// OS-neutral description of how a tool behaves when opening a Project, plus
/// whether it may appear in the project "open with" list at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenToolMeta {
    pub supports_project: bool,
    pub project_open_mode: ProjectOpenMode,
    pub folders_only: bool,
    pub project_args: Vec<String>,
    /// Extension allow-list (lowercase, no leading dot). Empty = applies to
    /// every selection (files and folders). Non-empty = only files whose
    /// extension is listed; the entry `"*"` matches any file. Directories
    /// never match a non-empty list. Filtering is presentation-level (the
    /// Navigator "Open with" submenu); launch itself stays permissive.
    pub file_extensions: Vec<String>,
}

impl OpenToolMeta {
    fn not_project() -> Self {
        Self {
            supports_project: false,
            project_open_mode: ProjectOpenMode::Workspace,
            folders_only: false,
            project_args: Vec::new(),
            file_extensions: Vec::new(),
        }
    }

    fn workspace_project(file_extensions: Vec<String>) -> Self {
        Self {
            supports_project: true,
            project_open_mode: ProjectOpenMode::Workspace,
            folders_only: false,
            project_args: Vec::new(),
            file_extensions,
        }
    }

    /// Project-incapable launcher scoped to the given extensions.
    fn file_scoped(extensions: &[&str]) -> Self {
        Self {
            supports_project: false,
            project_open_mode: ProjectOpenMode::Workspace,
            folders_only: false,
            project_args: Vec::new(),
            file_extensions: extensions.iter().map(|ext| ext.to_string()).collect(),
        }
    }
}

/// 7-Zip file manager browses/extracts archives AFM cannot (RAR/7z/…).
const SEVENZIP_FILE_EXTENSIONS: &[&str] = &[
    "zip", "7z", "rar", "tar", "gz", "tgz", "bz2", "xz", "zst", "iso", "cab",
];
/// Browsers open web documents; PDF/SVG render natively in the viewport.
const BROWSER_FILE_EXTENSIONS: &[&str] = &["html", "htm", "xhtml", "svg", "pdf"];
/// VLC covers the common audio/video containers.
const VLC_FILE_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mp3", "flac", "wav", "aac",
    "ogg", "opus", "m4a",
];

/// Built-in preset project behaviour, keyed by id. Anything not listed is
/// treated as a file-only launcher that never shows up under "open project".
pub fn preset_meta(id: &str) -> OpenToolMeta {
    match id {
        "vscode" | "cursor" | "trae" | "qwenwork" | "sublime" | "windsurf" | "idea"
        | "pycharm" | "webstorm" => OpenToolMeta::workspace_project(Vec::new()),
        // Qoder CN opens the workspace's CONTEXT.md so the agent picks up the
        // project brief instead of a bare folder.
        "qoder" => OpenToolMeta {
            supports_project: true,
            project_open_mode: ProjectOpenMode::ContextFile,
            folders_only: false,
            project_args: Vec::new(),
            file_extensions: Vec::new(),
        },
        // Any file, never a folder: Notepad++ has no folder view.
        "notepadpp" => OpenToolMeta::file_scoped(&["*"]),
        "sevenzip" => OpenToolMeta::file_scoped(SEVENZIP_FILE_EXTENSIONS),
        "chrome" | "edge" => OpenToolMeta::file_scoped(BROWSER_FILE_EXTENSIONS),
        "vlc" => OpenToolMeta::file_scoped(VLC_FILE_EXTENSIONS),
        _ => OpenToolMeta::not_project(),
    }
}

/// A user-defined open tool stored under `core.openTool.custom` as a JSON array.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomOpenTool {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub supports_project: bool,
    #[serde(default)]
    pub project_open_mode: ProjectOpenMode,
    #[serde(default)]
    pub folders_only: bool,
    #[serde(default)]
    pub project_args: Vec<String>,
    /// Extension allow-list; same semantics as [`OpenToolMeta::file_extensions`].
    #[serde(default)]
    pub file_extensions: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Setting key holding the JSON-encoded array of [`CustomOpenTool`]. It is a
/// plain string value so the schema's `String` coercion round-trips it; parsing
/// happens here so a malformed blob degrades to "no custom tools" instead of
/// failing the whole open-tools list.
pub const CUSTOM_TOOLS_KEY: &str = "core.openTool.custom";

/// Setting key holding a JSON array of open-tool ids that defines the order
/// used by the "Open with" menus (Navigator + Project) and the Settings list.
/// Empty / malformed / ids not present fall back to discovery order, so this is
/// purely an overlay the user can drag in Settings → Open tools.
pub const TOOL_ORDER_KEY: &str = "core.openTool.order";

pub fn ide_setting_enabled_key(id: &str) -> String {
    format!("core.openTool.{id}.enabled")
}

pub fn ide_setting_path_key(id: &str) -> String {
    format!("core.openTool.{id}.path")
}

/// Load + parse the persisted custom tools. Any read/parse failure yields an
/// empty list rather than an error so the Settings page never blanks out.
pub fn load_custom_tools(app: &AppHandle) -> Vec<CustomOpenTool> {
    let raw = crate::settings::read_string_setting(app, CUSTOM_TOOLS_KEY);
    parse_custom_tools(&raw)
}

fn parse_custom_tools(raw: &str) -> Vec<CustomOpenTool> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Vec<CustomOpenTool>>(trimmed) {
        Ok(tools) => tools,
        Err(_) => Vec::new(),
    }
}

/// Load + parse the persisted open-tool order (a JSON array of ids). Any read /
/// parse failure yields an empty order so menus fall back to discovery order.
pub fn load_open_tool_order(app: &AppHandle) -> Vec<String> {
    let raw = crate::settings::read_string_setting(app, TOOL_ORDER_KEY);
    parse_open_tool_order(&raw)
}

fn parse_open_tool_order(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    serde_json::from_str::<Vec<String>>(trimmed).unwrap_or_default()
}

/// Sort rank for `id`: its position in `order`, or `usize::MAX` when it is not
/// listed (so unlisted tools keep their discovery order at the end via a stable
/// sort). Extracted so the ordering rule is unit-testable without an AppHandle.
fn order_rank(order: &[String], id: &str) -> usize {
    order.iter().position(|entry| entry == id).unwrap_or(usize::MAX)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdeAppInfo {
    pub id: String,
    pub name: String,
    /// User toggle from Settings → Open tools (default on) or the custom record.
    pub enabled: bool,
    /// True when a launchable application was resolved (auto / configured / custom).
    pub available: bool,
    pub resolved_path: Option<String>,
    /// `auto` | `configured` | `custom` | `none`
    pub resolution: String,
    /// Configured override path from settings (may be empty).
    pub configured_path: String,
    // ---- shared project behaviour (file + project open use the same row) ----
    /// Built-in preset (`true`) vs a user-defined custom tool (`false`).
    pub builtin: bool,
    /// Whether this tool appears in the Project "open with" list.
    pub supports_project: bool,
    /// `workspace` | `contextFile`.
    pub project_open_mode: String,
    /// Only meaningful for folders/workspaces — hidden for plain files.
    pub folders_only: bool,
    /// Extra arguments appended before the path when opening a project.
    pub project_args: Vec<String>,
    /// Lowercase extension allow-list (no leading dot). Empty = applies to
    /// every selection; `"*"` = any file, never a directory.
    pub file_extensions: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdeLaunchResult {
    pub success: bool,
    pub error: Option<String>,
}

impl IdeLaunchResult {
    fn ok() -> Self {
        Self {
            success: true,
            error: None,
        }
    }
    fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
        }
    }
}

/// A resolved launch target together with its project behaviour. Custom tools
/// take precedence over presets when both share an id (they are user-owned).
struct ResolvedTool {
    target: LaunchTarget,
    meta: OpenToolMeta,
    enabled: bool,
}

fn configured_path_for(app: &AppHandle, id: &str) -> String {
    crate::settings::read_string_setting(app, &ide_setting_path_key(id))
}

fn enabled_for(app: &AppHandle, id: &str) -> bool {
    // Sanitized settings default to true; missing → treat as enabled.
    crate::settings::read_bool_setting_or(app, &ide_setting_enabled_key(id), true)
}

/// Resolve an id to a launchable target + metadata, for both file and project
/// open. Returns `None` when nothing is launchable; the `enabled` flag is
/// carried through so callers can distinguish "disabled" from "not installed".
fn resolve_tool(app: &AppHandle, id: &str) -> Option<ResolvedTool> {
    if let Some(custom) = load_custom_tools(app).into_iter().find(|t| t.id == id) {
        let target = platform::configured_launch_target(&custom.id, &custom.path)?;
        return Some(ResolvedTool {
            target,
            meta: OpenToolMeta {
                supports_project: custom.supports_project,
                project_open_mode: custom.project_open_mode,
                folders_only: custom.folders_only,
                project_args: custom.project_args,
                file_extensions: custom.file_extensions,
            },
            enabled: custom.enabled,
        });
    }

    let discovered = platform::ide_targets();
    if let Some(target) = discovered
        .iter()
        .find(|t| t.id == id && t.resolved_path.is_some())
    {
        return Some(ResolvedTool {
            target: target.clone(),
            meta: preset_meta(id),
            enabled: enabled_for(app, id),
        });
    }
    let configured = configured_path_for(app, id);
    if let Some(target) = platform::configured_launch_target(id, &configured) {
        return Some(ResolvedTool {
            target,
            meta: preset_meta(id),
            enabled: enabled_for(app, id),
        });
    }
    None
}

/// Project-facing capability snapshot used by `ai::open_editor`. `None` (from
/// [`project_capability`]) already encodes "unknown / disabled / not installed /
/// file-only", so the caller only needs to know *how* to open the workspace.
pub struct ProjectCapability {
    pub project_open_mode: ProjectOpenMode,
}

/// Report how `id` should open a Project. `None` when the id is unknown /
/// disabled / not installed / not project-capable.
pub fn project_capability(app: &AppHandle, id: &str) -> Option<ProjectCapability> {
    let tool = resolve_tool(app, id)?;
    if !tool.enabled || tool.target.resolved_path.is_none() || !tool.meta.supports_project {
        return None;
    }
    Some(ProjectCapability {
        project_open_mode: tool.meta.project_open_mode,
    })
}

/// List every open tool — built-in presets first (in platform order) then the
/// user's custom tools — with their resolved availability and project fields.
#[tauri::command]
pub fn ide_list_apps(app: AppHandle) -> Vec<IdeAppInfo> {
    let discovered = platform::ide_targets();
    let mut out: Vec<IdeAppInfo> = Vec::new();

    for preset in &discovered {
        let configured = configured_path_for(&app, &preset.id);
        let enabled = enabled_for(&app, &preset.id);
        let meta = preset_meta(&preset.id);
        let (resolved, resolution) = resolve_preset(&preset.id, &configured, &discovered);
        out.push(IdeAppInfo {
            id: preset.id.clone(),
            name: preset.name.clone(),
            enabled,
            available: resolved.is_some(),
            resolved_path: resolved.and_then(|t| t.resolved_path),
            resolution: resolution.to_string(),
            configured_path: configured,
            builtin: true,
            supports_project: meta.supports_project,
            project_open_mode: meta.project_open_mode.as_str().to_string(),
            folders_only: meta.folders_only,
            project_args: meta.project_args,
            file_extensions: meta.file_extensions,
        });
    }

    for custom in load_custom_tools(&app) {
        let target = platform::configured_launch_target(&custom.id, &custom.path);
        let available = target
            .as_ref()
            .and_then(|t| t.resolved_path.clone())
            .is_some();
        out.push(IdeAppInfo {
            id: custom.id.clone(),
            name: custom.name.clone(),
            enabled: custom.enabled,
            available,
            resolved_path: if available { Some(custom.path.clone()) } else { None },
            resolution: if available { "custom" } else { "none" }.to_string(),
            configured_path: custom.path.clone(),
            builtin: false,
            supports_project: custom.supports_project,
            project_open_mode: custom.project_open_mode.as_str().to_string(),
            folders_only: custom.folders_only,
            project_args: custom.project_args,
            file_extensions: custom.file_extensions,
        });
    }

    // Re-order by the user's persisted preference (stable: tools not present in
    // the order array keep their discovery order at the tail). This is the one
    // funnel that makes Navigator + Project "Open with" menus and the Settings
    // list share the same order (同源).
    let order = load_open_tool_order(&app);
    out.sort_by(|a, b| order_rank(&order, &a.id).cmp(&order_rank(&order, &b.id)));

    out
}

/// Prefer adapter discovery; a configured override is validated by the same
/// adapter (a Windows `.exe` is a file, a macOS `.app` is a directory).
fn resolve_preset(
    id: &str,
    configured_path: &str,
    discovered: &[LaunchTarget],
) -> (Option<LaunchTarget>, &'static str) {
    if let Some(target) = discovered
        .iter()
        .find(|t| t.id == id && t.resolved_path.is_some())
    {
        return (Some(target.clone()), "auto");
    }
    if let Some(target) = platform::configured_launch_target(id, configured_path) {
        return (Some(target), "configured");
    }
    (None, "none")
}

/// 仅自动发现（不读设置覆盖、不含自定义），供依赖体检使用。
pub fn ide_probe_auto() -> Vec<(String, bool, Option<String>)> {
    platform::ide_targets()
        .into_iter()
        .map(|target| {
            let available = target.resolved_path.is_some();
            (target.id, available, target.resolved_path)
        })
        .collect()
}

/// Open a file/folder in a resolved tool. Project-specific args are *not*
/// applied here (a plain file open ignores them); see [`ide_open_project_path`].
#[tauri::command]
pub fn ide_open_path(app: AppHandle, ide_id: String, path: String) -> IdeLaunchResult {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return IdeLaunchResult::err("Path is empty");
    }
    let target = Path::new(trimmed);
    if !target.exists() {
        return IdeLaunchResult::err(format!("Path not found: {trimmed}"));
    }

    let id = ide_id.trim();
    let Some(tool) = resolve_tool(&app, id) else {
        return IdeLaunchResult::err(format!("IDE not found: {id}"));
    };
    if !tool.enabled {
        return IdeLaunchResult::err(format!("IDE disabled in Settings: {id}"));
    }

    match platform::launch_ide(&tool.target, target, &[]) {
        Ok(()) => IdeLaunchResult::ok(),
        Err(error) => IdeLaunchResult::err(error),
    }
}

/// Open a resolved workspace/file in a project-capable tool, appending that
/// tool's project-specific arguments. Used by `ai::open_editor` after it has
/// turned the workspace path into the concrete file/dir to hand over.
pub fn ide_open_project_path(app: &AppHandle, ide_id: &str, path: &str) -> IdeLaunchResult {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return IdeLaunchResult::err("Path is empty");
    }
    let target = Path::new(trimmed);
    if !target.exists() {
        return IdeLaunchResult::err(format!("Path not found: {trimmed}"));
    }
    let id = ide_id.trim();
    let Some(tool) = resolve_tool(app, id) else {
        return IdeLaunchResult::err(format!("IDE not found: {id}"));
    };
    if !tool.enabled {
        return IdeLaunchResult::err(format!("IDE disabled in Settings: {id}"));
    }
    if !tool.meta.supports_project {
        return IdeLaunchResult::err(format!("IDE does not support opening projects: {id}"));
    }
    match platform::launch_ide(&tool.target, target, &tool.meta.project_args) {
        Ok(()) => IdeLaunchResult::ok(),
        Err(error) => IdeLaunchResult::err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn discovered() -> Vec<LaunchTarget> {
        platform::ide_targets()
    }

    #[test]
    fn lists_preset_ides_including_qoder_and_qwenwork() {
        // Platform-specific id lists are pinned per-adapter (platform/windows.rs
        // `ide_targets_list_the_fifteen_open_tool_presets`, platform/macos.rs
        // `targets_and_launch_contract`) plus scripts/check-launch-targets.mjs;
        // this stays adapter-neutral (7.20: no cfg in the thin wire module).
        let ids: Vec<_> = discovered().into_iter().map(|t| t.id).collect();
        for id in [
            "vscode", "cursor", "trae", "qoder", "qwenwork", "sublime", "idea", "pycharm",
            "webstorm",
        ] {
            assert!(ids.contains(&id.to_string()), "{id} missing from presets");
        }
        assert_eq!(
            discovered().iter().find(|t| t.id == "qoder").unwrap().name,
            "Qoder CN"
        );
    }

    #[test]
    fn preset_meta_marks_agent_ides_project_capable() {
        assert!(preset_meta("vscode").supports_project);
        assert_eq!(preset_meta("qoder").project_open_mode, ProjectOpenMode::ContextFile);
        assert!(preset_meta("qwenwork").supports_project);
        assert!(!preset_meta("wt").supports_project);
        // New project-capable presets open workspaces like VS Code.
        for id in ["sublime", "windsurf", "idea", "pycharm", "webstorm"] {
            assert!(preset_meta(id).supports_project, "{id} should be project-capable");
            assert!(preset_meta(id).file_extensions.is_empty(), "{id} is unscoped");
        }
        // Project-incapable presets are extension-scoped instead.
        assert_eq!(preset_meta("notepadpp").file_extensions, vec!["*".to_string()]);
        assert!(!preset_meta("notepadpp").supports_project);
        assert!(preset_meta("sevenzip").file_extensions.contains(&"zip".to_string()));
        assert!(preset_meta("sevenzip").file_extensions.contains(&"rar".to_string()));
        assert_eq!(
            preset_meta("chrome").file_extensions,
            preset_meta("edge").file_extensions
        );
        assert!(preset_meta("vlc").file_extensions.contains(&"mp4".to_string()));
        assert!(preset_meta("vlc").file_extensions.contains(&"flac".to_string()));
    }

    #[test]
    fn custom_tools_parse_and_tolerate_garbage() {
        let json = r#"[{"id":"custom:foo","name":"Foo","path":"C:\\foo.exe","supportsProject":true,"projectOpenMode":"contextFile","foldersOnly":true,"projectArgs":["--agent"],"fileExtensions":["zip","rar"]}]"#;
        let tools = parse_custom_tools(json);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].id, "custom:foo");
        assert!(tools[0].enabled); // defaulted true
        assert!(tools[0].supports_project);
        assert_eq!(tools[0].project_open_mode, ProjectOpenMode::ContextFile);
        assert_eq!(tools[0].project_args, vec!["--agent".to_string()]);
        assert_eq!(tools[0].file_extensions, vec!["zip".to_string(), "rar".to_string()]);
        // Legacy records without fileExtensions default to unscoped.
        let legacy = parse_custom_tools(r#"[{"id":"custom:bar","name":"Bar","path":"C:\\bar.exe"}]"#);
        assert!(legacy[0].file_extensions.is_empty());
        // Garbage / empty degrade to none, never panic.
        assert!(parse_custom_tools("").is_empty());
        assert!(parse_custom_tools("not json").is_empty());
    }

    #[test]
    fn empty_configured_path_never_resolves() {
        let (resolved, resolution) = resolve_preset("___missing_ide___", "", &discovered());
        assert!(resolved.is_none());
        assert_eq!(resolution, "none");
    }

    #[test]
    fn configured_override_is_validated_by_the_adapter() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let file = std::env::temp_dir().join(format!("afm-ide-override-{stamp}"));
        fs::write(&file, b"x").unwrap();
        let (resolved, resolution) =
            resolve_preset("___missing_ide___", file.to_str().unwrap(), &discovered());
        assert_eq!(resolution, "configured");
        assert_eq!(resolved.unwrap().resolved_path.as_deref(), file.to_str());
        let _ = fs::remove_file(&file);
    }

    #[test]
    fn setting_key_helpers() {
        assert_eq!(ide_setting_enabled_key("cursor"), "core.openTool.cursor.enabled");
        assert_eq!(ide_setting_path_key("vscode"), "core.openTool.vscode.path");
        assert_eq!(CUSTOM_TOOLS_KEY, "core.openTool.custom");
        assert_eq!(TOOL_ORDER_KEY, "core.openTool.order");
    }

    #[test]
    fn open_tool_order_parses_and_ranks() {
        // Malformed / empty degrade to no preference.
        assert!(parse_open_tool_order("").is_empty());
        assert!(parse_open_tool_order("nope").is_empty());
        let order = parse_open_tool_order(r#"["cursor","qoder","vscode"]"#);
        assert_eq!(order, vec!["cursor", "qoder", "vscode"]);
        // Listed ids rank by position; unlisted fall to the tail (usize::MAX).
        assert_eq!(order_rank(&order, "cursor"), 0);
        assert_eq!(order_rank(&order, "vscode"), 2);
        assert_eq!(order_rank(&order, "trae"), usize::MAX);
    }
}
