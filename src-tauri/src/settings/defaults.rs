// SPDX-License-Identifier: GPL-3.0-or-later

use super::platform_policy;
use super::types::{
    SettingDefinition, SettingValueKind, SettingsDocument, SettingsSchemaManifest, SettingsTier,
    SETTINGS_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub fn schema_manifest() -> SettingsSchemaManifest {
    SettingsSchemaManifest {
        version: SETTINGS_SCHEMA_VERSION,
        tiers: vec![
            SettingsTier::Core,
            SettingsTier::Power,
            SettingsTier::Experimental,
        ],
        definitions: setting_definitions(),
    }
}

/// Platform-aware definitions: the same policy feeds the schema sent to the
/// UI and the sanitizer applied to the persisted document.
/// Appends the dynamic overlay of currently-enabled Feature Pack `config[]`
/// declarations (built-in keys win — the overlay never overrides or duplicates
/// them) so pack-declared settings survive `sanitize_document` and persist.
pub fn setting_definitions() -> Vec<SettingDefinition> {
    let caps = crate::platform::current().capabilities();
    let mut definitions = raw_setting_definitions();
    for definition in &mut definitions {
        platform_policy::apply(definition, &caps);
    }
    let existing: std::collections::BTreeSet<String> =
        definitions.iter().map(|definition| definition.key.clone()).collect();
    for definition in super::overlay::overlay_definitions() {
        if !existing.contains(definition.key.as_str()) {
            definitions.push(definition);
        }
    }
    definitions
}

/// Host-independent declarations. Tests inject synthetic capabilities here.
pub(crate) fn raw_setting_definitions() -> Vec<SettingDefinition> {
    vec![
        def(
            "core.language",
            SettingsTier::Core,
            "Language",
            "UI language: en | zh-CN | zh-TW | ja | de | fr | system",
            SettingValueKind::Enum,
            Some(vec![
                "en".to_string(),
                "zh-CN".to_string(),
                "zh-TW".to_string(),
                "ja".to_string(),
                "de".to_string(),
                "fr".to_string(),
                "system".to_string(),
            ]),
            json!("system"),
        ),
        def(
            "core.theme",
            SettingsTier::Core,
            "Theme",
            "Color theme: light | dark | system",
            SettingValueKind::Enum,
            Some(vec![
                "light".to_string(),
                "dark".to_string(),
                "system".to_string(),
            ]),
            json!("system"),
        ),
        def(
            "core.defaultLocation",
            SettingsTier::Core,
            "Default location",
            "OS path opened by the Home button (default: resolved home directory)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.projectsRoot",
            SettingsTier::Core,
            "Projects Root",
            "Folder that holds Project Workspaces (default: app config projects dir)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.startupPage",
            SettingsTier::Core,
            "Startup page",
            "First page after launch: navigator | projects | discovery",
            SettingValueKind::Enum,
            Some(vec![
                "navigator".to_string(),
                "projects".to_string(),
                "discovery".to_string(),
            ]),
            json!("navigator"),
        ),
        def(
            "core.showHidden",
            SettingsTier::Core,
            "Show hidden items",
            "Include hidden files/folders in Kernel listings",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "core.listDensity",
            SettingsTier::Core,
            "List density",
            "File list density; chrome stays Standard. Default: standard",
            SettingValueKind::Enum,
            Some(vec!["standard".to_string(), "compact".to_string()]),
            json!("standard"),
        ),
        // FIL-59：发现/扩展归入 Power（键名不变，仅展示分层）。
        def(
            "core.everythingPreferred",
            SettingsTier::Power,
            "Everything preferred",
            "Prefer Everything IPC for Discovery when available",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "core.extensionsEnabled",
            SettingsTier::Power,
            "Extensions enabled",
            "Master switch for the extension host",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "core.launchAtStartup",
            SettingsTier::Core,
            "Launch at startup",
            "Start A File Manager when you sign in",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "core.launchAtStartupHidden",
            SettingsTier::Core,
            "Start hidden to tray",
            "When launched at login, hide the main window to the system tray",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "core.closeBehavior",
            SettingsTier::Core,
            "When closing the window",
            "Ask each time, hide to the system tray, or quit the app",
            SettingValueKind::Enum,
            Some(vec![
                "ask".to_string(),
                "tray".to_string(),
                "quit".to_string(),
            ]),
            json!("ask"),
        ),
        def(
            "core.discoveryBlacklist",
            SettingsTier::Core,
            "Discovery directory blacklist",
            "Directories excluded from Discovery search results (one path per line in Settings)",
            SettingValueKind::StringArray,
            None,
            json!([]),
        ),
        def(
            "core.newFolderDefaultName",
            SettingsTier::Core,
            "Default new folder name",
            "Suggested name when creating a folder (default: current language name)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.newFileDefaultName",
            SettingsTier::Core,
            "Default new file name",
            "Suggested name when creating a file (default: current language name)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.fileDoubleClickAction",
            SettingsTier::Core,
            "File double-click action",
            "What happens when double-clicking a file in the Navigator (AIA-118)",
            SettingValueKind::Enum,
            Some(vec![
                "systemDefault".into(),
                "openWithDialog".into(),
                "reveal".into(),
                "vscode".into(),
                "cursor".into(),
                "trae".into(),
                "qoder".into(),
                "none".into(),
            ]),
            json!("systemDefault"),
        ),
        def(
            "core.navigatorViewMode",
            SettingsTier::Core,
            "Navigator view",
            "Icon, list, details, tiles or content view",
            SettingValueKind::Enum,
            Some(vec![
                "iconsExtraLarge".to_string(),
                "iconsLarge".to_string(),
                "iconsMedium".to_string(),
                "iconsSmall".to_string(),
                "list".to_string(),
                "details".to_string(),
                "tiles".to_string(),
                "content".to_string(),
            ]),
            json!("details"),
        ),
        // v17：布尔分组开关由 core.navigatorGroupBy 枚举取代；键定义保留，
        // 迁移读它，sanitize 不丢弃旧文档里的残留值（ADR 0030）。
        def(
            "core.navigatorGroupByType",
            SettingsTier::Core,
            "Group by type (legacy)",
            "Pre-v17 boolean grouping switch; migrated into core.navigatorGroupBy",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "core.navigatorSortColumn",
            SettingsTier::Core,
            "Sort by",
            "Navigator list sort field: name | size | modified | type",
            SettingValueKind::Enum,
            Some(vec![
                "name".to_string(),
                "size".to_string(),
                "modified".to_string(),
                "type".to_string(),
            ]),
            json!("name"),
        ),
        def(
            "core.navigatorSortDirection",
            SettingsTier::Core,
            "Sort direction",
            "Navigator list sort direction: ascending | descending",
            SettingValueKind::Enum,
            Some(vec!["asc".to_string(), "desc".to_string()]),
            json!("asc"),
        ),
        def(
            "core.navigatorGroupBy",
            SettingsTier::Core,
            "Group by",
            "Navigator list grouping field: none | name | size | modified | type",
            SettingValueKind::Enum,
            Some(vec![
                "none".to_string(),
                "name".to_string(),
                "size".to_string(),
                "modified".to_string(),
                "type".to_string(),
            ]),
            json!("none"),
        ),
        def(
            "core.navigatorGroupDirection",
            SettingsTier::Core,
            "Group direction",
            "Navigator group bucket order: ascending | descending",
            SettingValueKind::Enum,
            Some(vec!["asc".to_string(), "desc".to_string()]),
            json!("asc"),
        ),
        // ADR 0031: drill depth level (user preference). Whether drill-through
        // is currently on is session state and is never persisted.
        def(
            "core.navigatorDrillDepth",
            SettingsTier::Core,
            "Drill-through depth",
            "How many levels of subfolders the Navigator flattens when drill-through is on: 1..5",
            SettingValueKind::Number,
            None,
            json!(2),
        ),
        def(
            "core.newTabBehavior",
            SettingsTier::Core,
            "New tab behavior",
            "What the + new tab control opens: guide page or duplicate the active tab",
            SettingValueKind::Enum,
            Some(vec!["guide".to_string(), "duplicate".to_string()]),
            json!("guide"),
        ),
        def(
            "core.trackRecentVisits",
            SettingsTier::Core,
            "Record recent visits",
            "Track folders you open to build the Recent visits list",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        // Open tools (IDE/Agent) — edited in dedicated Settings section; filtered from Core list.
        def(
            "core.openTool.vscode.enabled",
            SettingsTier::Core,
            "Visual Studio Code",
            "Show VS Code in Open with when available",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "core.openTool.vscode.path",
            SettingsTier::Core,
            "VS Code path",
            "Optional VS Code app path when auto-discovery fails",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.openTool.cursor.enabled",
            SettingsTier::Core,
            "Cursor",
            "Show Cursor in Open with when available",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "core.openTool.cursor.path",
            SettingsTier::Core,
            "Cursor path",
            "Optional Cursor app path when auto-discovery fails",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.openTool.trae.enabled",
            SettingsTier::Core,
            "Trae",
            "Show Trae in Open with when available",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "core.openTool.trae.path",
            SettingsTier::Core,
            "Trae path",
            "Optional Trae app path when auto-discovery fails",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.openTool.qoder.enabled",
            SettingsTier::Core,
            "Qoder CN",
            "Show Qoder CN in Open with when available",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "core.openTool.qoder.path",
            SettingsTier::Core,
            "Qoder CN path",
            "Optional Qoder CN app path when auto-discovery fails",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.openTool.qwenwork.enabled",
            SettingsTier::Core,
            "QwenWork",
            "Show QwenWork in Open with when available",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "core.openTool.qwenwork.path",
            SettingsTier::Core,
            "QwenWork path",
            "Optional QwenWorkCN.exe path when auto-discovery fails",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.openTool.custom",
            SettingsTier::Power,
            "Custom open tools",
            "JSON array of user-defined open tools (id/name/path/project attributes)",
            SettingValueKind::String,
            None,
            json!("[]"),
        ),
        def(
            "core.openTool.order",
            SettingsTier::Power,
            "Open tool order",
            "JSON array of open-tool ids ordering the \"Open with\" menus and the Settings list (unlisted ids fall back to discovery order)",
            SettingValueKind::String,
            None,
            json!("[]"),
        ),
        def(
            "power.confirmBeforeDelete",
            SettingsTier::Power,
            "Confirm before delete",
            "Ask before permanent delete operations",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "power.perPathOverridesEnabled",
            SettingsTier::Power,
            "Per-path overrides",
            "Allow per-path navigator defaults to override group/global",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "power.providerPreloadKnowledge",
            SettingsTier::Power,
            "Preload knowledge provider",
            "Warm awam://knowledge during provider preload (disable if the knowledge root is huge)",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "power.awamGatewayUrl",
            SettingsTier::Power,
            "awam-file gateway URL",
            "HTTP base URL for the awam-file resource gateway (default http://127.0.0.1:18787)",
            SettingValueKind::String,
            None,
            json!("http://127.0.0.1:18787"),
        ),
        def(
            "power.awamKnowledgeServiceUrl",
            SettingsTier::Power,
            "awam-knowledge service URL",
            "HTTP base URL for the awam-knowledge content/vault service used by the search palette (default http://127.0.0.1:18790)",
            SettingValueKind::String,
            None,
            json!("http://127.0.0.1:18790"),
        ),
        def(
            "power.awamKnowledgeServiceToken",
            SettingsTier::Power,
            "awam-knowledge service token",
            "Bearer token for the awam-knowledge service (KM_SERVICE_TOKEN; empty disables authenticated calls)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "power.awamKnowledgeDataRoot",
            SettingsTier::Power,
            "awam-knowledge data root",
            "Absolute path to the sanitized knowledge working copy root (KM_DATA_ROOT). Empty = defer to the knowledge service's own settings.yaml. AFM injects this into the managed awam-knowledge service on spawn.",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "power.awamKnowledgeSourceId",
            SettingsTier::Power,
            "awam-knowledge source id",
            "Source id under the data root to index (KM_SOURCE_ID); index root = <data root>/<source id>. Empty = the service default.",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "power.awamKnowledgeSecretScript",
            SettingsTier::Power,
            "awam-knowledge vault script",
            "Absolute path to agent_secret.py (KM_SECRET_SCRIPT) enabling the managed knowledge service's vault channel; empty disables vault reveal/copy.",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "power.awamKnowledgeSecretAgentToken",
            SettingsTier::Power,
            "awam-knowledge vault agent token",
            "Human-channel caller token for agent_secret.py (KM_SECRET_AGENT_TOKEN); empty disables the vault channel.",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "power.awamKnowledgeSecretId",
            SettingsTier::Power,
            "awam-knowledge vault source id",
            "Desensitization source id passed to agent_secret.py (KM_SECRET_ID); empty disables the vault channel.",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "power.voiceSearchAutoStart",
            SettingsTier::Power,
            "Voice search auto start",
            "Start voice capture automatically when the search palette opens (AI Add-on voice input)",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "power.sttBaseUrl",
            SettingsTier::Power,
            "Speech-to-text base URL",
            "OpenAI-compatible transcription base URL; POST {baseUrl}/audio/transcriptions (e.g. https://api.openai.com/v1)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "power.sttModel",
            SettingsTier::Power,
            "Speech-to-text model",
            "Model name sent to the transcription endpoint (e.g. whisper-1)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "power.sttApiKey",
            SettingsTier::Power,
            "Speech-to-text API key",
            "Bearer key for the transcription endpoint (stored in settings.json; empty disables voice input)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "experimental.debugStatusInTitlebar",
            SettingsTier::Experimental,
            "Debug status in titlebar",
            "Show phase/debug caption in the custom titlebar (hidden in Settings UI; key retained)",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "experimental.previewPane",
            SettingsTier::Experimental,
            "Preview pane",
            "Show image and plain-text preview beside the Explorer list",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "preview.externalProvider",
            SettingsTier::Experimental,
            "Preview bridge provider",
            "Space uses QuickLook (named pipe) on Windows, PowerToys Peek, or the built-in pane. On macOS it uses the system Quick Look. Missing a bridge does not hard-fail.",
            SettingValueKind::Enum,
            Some(vec![
                "auto".to_string(),
                "peek".to_string(),
                "quicklook".to_string(),
                "builtin".to_string(),
                "off".to_string(),
            ]),
            json!("auto"),
        ),
    ]
}

fn def(
    key: &str,
    tier: SettingsTier,
    title: &str,
    description: &str,
    value_kind: SettingValueKind,
    enum_values: Option<Vec<String>>,
    default: Value,
) -> SettingDefinition {
    SettingDefinition {
        key: key.to_string(),
        tier,
        title: title.to_string(),
        description: description.to_string(),
        value_kind,
        enum_values,
        default,
        hidden: false,
    }
}

pub fn default_document() -> SettingsDocument {
    let mut values = BTreeMap::new();
    for definition in setting_definitions() {
        values.insert(definition.key, definition.default);
    }
    SettingsDocument {
        version: SETTINGS_SCHEMA_VERSION,
        values,
    }
}

pub fn definition_map() -> BTreeMap<String, SettingDefinition> {
    setting_definitions()
        .into_iter()
        .map(|definition| (definition.key.clone(), definition))
        .collect()
}
