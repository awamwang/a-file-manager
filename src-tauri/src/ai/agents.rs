// SPDX-License-Identifier: GPL-3.0-or-later
//! Agent Target registry (AIA-215 / A2 · ADR 0006).
//!
//! Unified targets for Skills dirs + MCP config paths (global + project-relative).
//! `mcp_usage_prompt` is a **placeholder** — stored, never injected (A2).
//! Discovery probes path existence; enable/custom are user actions. No Skill/MCP deploy (A3+/A5).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const AI_CONFIG_FILE_NAME: &str = "ai-config.json";
/// One-shot backup of unparseable non-empty `ai-config.json` (AIA-277).
pub const AI_CONFIG_CORRUPT_BACKUP_NAME: &str = "ai-config.json.corrupt.bak";
/// v4：Unix 宿主把内置预设的 Windows 路径字面量归一化成 `~/...`（展示修复）。
/// v3：Agent Target 增加 sessions / rules 只读路径槽（ADR 0028）。v2：`defaultInProjects` + `skillDeployMode`（ADR 0010 / 0011）。
pub const AI_CONFIG_SCHEMA_VERSION: u32 = 4;

pub const SKILL_DEPLOY_MODE_JUNCTION: &str = "junction";
pub const SKILL_DEPLOY_MODE_COPY: &str = "copy";

pub const ERR_AGENT_ID_EMPTY: &str = "AI_AGENT_ID_EMPTY";
pub const ERR_AGENT_ID_INVALID: &str = "AI_AGENT_ID_INVALID";
pub const ERR_AGENT_NAME_EMPTY: &str = "AI_AGENT_NAME_EMPTY";
pub const ERR_AGENT_UNKNOWN: &str = "AI_AGENT_UNKNOWN";
pub const ERR_AGENT_BUILTIN_REMOVE: &str = "AI_AGENT_BUILTIN_REMOVE";
pub const ERR_AGENT_ID_CONFLICT: &str = "AI_AGENT_ID_CONFLICT";
pub const ERR_AGENT_SLOT_UNKNOWN: &str = "AI_AGENT_SLOT_UNKNOWN";
pub const ERR_AGENT_PATH_EMPTY: &str = "AI_AGENT_PATH_EMPTY";
pub const ERR_AGENT_PATH_NOT_FOUND: &str = "AI_AGENT_PATH_NOT_FOUND";

/// ADR 0028：`ai_agent_target_open_path` 的全局路径槽位名（与 FE types 对齐）。
pub const SLOT_GLOBAL_SKILLS_DIR: &str = "globalSkillsDir";
pub const SLOT_GLOBAL_SESSIONS_DIR: &str = "globalSessionsDir";
pub const SLOT_GLOBAL_RULES_PATH: &str = "globalRulesPath";
pub const SLOT_GLOBAL_MCP_PATH: &str = "globalMcpPath";

const MAX_ID_LEN: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentTarget {
    pub id: String,
    pub display_name: String,
    /// `"builtin"` | `"custom"`.
    pub kind: String,
    pub enabled: bool,
    /// 新项目「项目 Agent 启用集」的默认勾选（ADR 0010）。缺省迁移时对齐 `enabled`。
    #[serde(default)]
    pub default_in_projects: bool,
    /// True when global skills dir and/or global MCP path exists on disk.
    pub discovered: bool,
    pub global_skills_dir: String,
    pub project_skills_dir: String,
    pub global_mcp_path: String,
    pub project_mcp_path: String,
    /// 会话/转录目录（只读展示与跳转，ADR 0028）。
    #[serde(default)]
    pub global_sessions_dir: String,
    /// 全局规则/记忆文件或目录（只读展示与跳转，ADR 0028）。
    #[serde(default)]
    pub global_rules_path: String,
    /// 项目相对规则目录（只读展示与跳转，ADR 0028）。
    #[serde(default)]
    pub project_rules_dir: String,
    /// Placeholder only — not injected into agents (ADR 0006 / A2).
    pub mcp_usage_prompt: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentTargetPatch {
    pub display_name: Option<String>,
    pub enabled: Option<bool>,
    pub default_in_projects: Option<bool>,
    pub global_skills_dir: Option<String>,
    pub project_skills_dir: Option<String>,
    pub global_mcp_path: Option<String>,
    pub project_mcp_path: Option<String>,
    pub global_sessions_dir: Option<String>,
    pub global_rules_path: Option<String>,
    pub project_rules_dir: Option<String>,
    pub mcp_usage_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiConfigDocument {
    pub schema_version: u32,
    pub agent_targets: Vec<AgentTarget>,
    /// Empty → default `<config_dir>\.agents\skills` (ADR 0005 / A3).
    #[serde(default)]
    pub skills_library_root: String,
    /// Public MCP catalog (ADR 0006 / A5) — APPDATA only, not per-Workspace.
    #[serde(default)]
    pub mcp_catalog: Vec<McpCatalogEntry>,
    /// AI model Providers MVP (A7) — presets + Compatible; not FS Providers.
    #[serde(default)]
    pub ai_providers: Vec<crate::ai::providers::AiProvider>,
    /// Skill 部署方式：`junction`（默认）| `copy`（ADR 0011）。
    #[serde(default = "default_skill_deploy_mode")]
    pub skill_deploy_mode: String,
}

fn default_skill_deploy_mode() -> String {
    SKILL_DEPLOY_MODE_JUNCTION.to_string()
}

pub fn normalize_skill_deploy_mode(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        SKILL_DEPLOY_MODE_COPY => SKILL_DEPLOY_MODE_COPY.to_string(),
        _ => SKILL_DEPLOY_MODE_JUNCTION.to_string(),
    }
}

/// One public MCP server entry (stored in APPDATA AI config).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpCatalogEntry {
    pub id: String,
    pub display_name: String,
    /// Opaque JSON object text for the server config (command/args/url/env…).
    pub server_json: String,
    pub created_at: u64,
    pub updated_at: u64,
}

impl AiConfigDocument {
    pub fn empty() -> Self {
        Self {
            schema_version: AI_CONFIG_SCHEMA_VERSION,
            agent_targets: Vec::new(),
            skills_library_root: String::new(),
            mcp_catalog: Vec::new(),
            ai_providers: Vec::new(),
            skill_deploy_mode: default_skill_deploy_mode(),
        }
    }
}

struct BuiltinDef {
    id: &'static str,
    display_name: &'static str,
    global_skills_dir: &'static str,
    project_skills_dir: &'static str,
    global_mcp_path: &'static str,
    project_mcp_path: &'static str,
}

/// ADR 0028：sessions / rules 扩展路径槽的内置补充表（与 60 条 `BUILTIN_DEFS` 分离，避免全量改写）。
/// 仅收录有公开约定路径的 Agent；未列出的预设三槽为空（用户可自行补填）。
/// (id, global_sessions_dir, global_rules_path, project_rules_dir)
const BUILTIN_EXTRA_PATHS: &[(&str, &str, &str, &str)] = &[
    ("claude-code", r"%USERPROFILE%\.claude\projects", r"%USERPROFILE%\.claude\CLAUDE.md", r".claude"),
    ("codex", r"~/.codex/sessions", r"~/.codex/AGENTS.md", ""),
    ("gemini-cli", r"~/.gemini/tmp", r"~/.gemini/GEMINI.md", ""),
    ("qwen-code", "", r"~/.qwen/QWEN.md", ""),
    ("cursor", "", r"%USERPROFILE%\.cursor\rules", r".cursor\rules"),
    ("trae", "", r"%USERPROFILE%\.trae\rules", r".trae\rules"),
    ("trae-cn", "", r"~/.trae-cn/rules", r".trae/rules"),
    ("windsurf", "", "", r".windsurf/rules"),
    ("kilo-code", "", "", r".kilocode/rules"),
    ("roo-code", "", "", r".roo/rules"),
    ("opencode", "", r"%USERPROFILE%\.config\opencode\AGENTS.md", ""),
    ("goose", r"~/.config/goose/sessions", "", ""),
];

fn builtin_extra_paths(id: &str) -> (&'static str, &'static str, &'static str) {
    for (extra_id, sessions, global_rules, project_rules) in BUILTIN_EXTRA_PATHS {
        if *extra_id == id {
            return (sessions, global_rules, project_rules);
        }
    }
    ("", "", "")
}

/// Built-in Agent Target presets (path conventions from ADR 0006 + local agent layouts).
/// Paths use `%USERPROFILE%` / `%APPDATA%` tokens expanded at discovery time.
const BUILTIN_DEFS: &[BuiltinDef] = &[
    BuiltinDef {
        id: "cursor",
        display_name: "Cursor",
        global_skills_dir: r"%USERPROFILE%\.cursor\skills",
        project_skills_dir: r".cursor\skills",
        global_mcp_path: r"%USERPROFILE%\.cursor\mcp.json",
        project_mcp_path: r".cursor\mcp.json",
    },
    BuiltinDef {
        id: "claude-code",
        display_name: "Claude Code",
        global_skills_dir: r"%USERPROFILE%\.claude\skills",
        project_skills_dir: r".claude\skills",
        global_mcp_path: r"%USERPROFILE%\.claude.json",
        project_mcp_path: r".mcp.json",
    },
    BuiltinDef {
        id: "cline",
        display_name: "Cline / agents",
        global_skills_dir: r"%USERPROFILE%\.agents\skills",
        project_skills_dir: r".agents\skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "opencode",
        display_name: "OpenCode",
        global_skills_dir: r"%USERPROFILE%\.config\opencode\skills",
        project_skills_dir: r".opencode\skills",
        global_mcp_path: r"%USERPROFILE%\.config\opencode\opencode.json",
        project_mcp_path: r".opencode\opencode.json",
    },
    BuiltinDef {
        id: "trae",
        display_name: "Trae",
        global_skills_dir: r"%USERPROFILE%\.trae\skills",
        project_skills_dir: r".trae\skills",
        global_mcp_path: r"%USERPROFILE%\.trae\mcp.json",
        project_mcp_path: r".trae\mcp.json",
    },
    BuiltinDef {
        id: "qoder",
        display_name: "Qoder",
        global_skills_dir: r"%USERPROFILE%\.qoder\skills",
        project_skills_dir: r".qoder\skills",
        global_mcp_path: r"%USERPROFILE%\.qoder\mcp.json",
        project_mcp_path: r".qoder\mcp.json",
    },
    // QwenWork (QwenWorkCN desktop, e.g. `C:\Program Files\QwenWorkCN\<ver>\QwenWorkCN.exe`).
    // Global skills live under the app home `~/.qwenworkcn/skills`. MCP servers are managed
    // inside the app (no user-editable global/project `mcp.json` file), so MCP paths are empty
    // and only the skills directory drives discovery + deploy — same shape as the `cline` preset.
    BuiltinDef {
        id: "qwenwork",
        display_name: "QwenWork",
        global_skills_dir: r"%USERPROFILE%\.qwenworkcn\skills",
        project_skills_dir: r".qwenworkcn\skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "workbuddy",
        display_name: "WorkBuddy",
        global_skills_dir: r"%USERPROFILE%\.workbuddy\skills",
        project_skills_dir: r".workbuddy\skills",
        global_mcp_path: r"%USERPROFILE%\.workbuddy\mcp.json",
        project_mcp_path: r".workbuddy\mcp.json",
    },
    BuiltinDef {
        id: "vscode",
        display_name: "VS Code / Continue",
        global_skills_dir: r"%USERPROFILE%\.continue\skills",
        project_skills_dir: r".continue\skills",
        global_mcp_path: r"%USERPROFILE%\.continue\config.json",
        project_mcp_path: r".continue\config.json",
    },
    // ------------------------------------------------------------------
    // 扩展内置清单（2026-11 单次批量收录，来源：skills-manager v1.44 的
    // `default_tool_adapters()` 静态注册表，路径约定以其 CLI `agents list`
    // 实测输出为准）。该项目无远程清单更新机制——清单随版本编译，AFM 同理：
    // `sync_builtins_and_discover()` 会在加载时把缺失的内置预设补种进
    // ai-config.json，因此本批新增对存量配置零迁移成本。
    // 新增预设路径统一用 `~` + `/` 记法（`expand_agent_path` 原生支持，
    // 与上方 `%USERPROFILE%` 反斜杠记法等价）。MCP 路径一律留空：
    // skills-manager 不管理 MCP，其注册表不含可靠的全局/项目 MCP 文件
    // 约定，留空表示「仅 Skills 部署/发现」，与 `cline`/`qwenwork` 预设
    // 同一形态；需要 MCP 的用户可自行在详情页补路径。
    // ------------------------------------------------------------------
    BuiltinDef {
        id: "codex",
        display_name: "Codex",
        global_skills_dir: r"~/.codex/skills",
        project_skills_dir: r".codex/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "github-copilot",
        display_name: "GitHub Copilot",
        global_skills_dir: r"~/.copilot/skills",
        project_skills_dir: r".copilot/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "gemini-cli",
        display_name: "Gemini CLI",
        global_skills_dir: r"~/.gemini/skills",
        project_skills_dir: r".gemini/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        // Google Antigravity：全局 Skills 在 `~/.gemini/config/skills`，
        // 工作区级在 `<ws>/.agents/skills`（skills-manager 同源注释）。
        id: "antigravity",
        display_name: "Antigravity",
        global_skills_dir: r"~/.gemini/config/skills",
        project_skills_dir: r".agents/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "grok",
        display_name: "Grok",
        global_skills_dir: r"~/.grok/skills",
        project_skills_dir: r".grok/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "windsurf",
        display_name: "Windsurf",
        global_skills_dir: r"~/.codeium/windsurf/skills",
        project_skills_dir: r".codeium/windsurf/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "goose",
        display_name: "Goose",
        global_skills_dir: r"~/.config/goose/skills",
        project_skills_dir: r".config/goose/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "amp",
        display_name: "Amp",
        global_skills_dir: r"~/.config/agents/skills",
        project_skills_dir: r".config/agents/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "kilo-code",
        display_name: "Kilo Code",
        global_skills_dir: r"~/.kilocode/skills",
        project_skills_dir: r".kilocode/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "roo-code",
        display_name: "Roo Code",
        global_skills_dir: r"~/.roo/skills",
        project_skills_dir: r".roo/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "droid",
        display_name: "Droid",
        global_skills_dir: r"~/.factory/skills",
        project_skills_dir: r".factory/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "openclaw",
        display_name: "OpenClaw",
        global_skills_dir: r"~/.openclaw/skills",
        project_skills_dir: r".openclaw/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "openhands",
        display_name: "OpenHands",
        global_skills_dir: r"~/.openhands/skills",
        project_skills_dir: r".openhands/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "hermes",
        display_name: "Hermes Agent",
        global_skills_dir: r"~/.hermes/skills",
        project_skills_dir: r".hermes/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "deepagents",
        display_name: "Deep Agents",
        global_skills_dir: r"~/.deepagents/agent/skills",
        project_skills_dir: r".deepagents/agent/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "firebender",
        display_name: "Firebender",
        global_skills_dir: r"~/.firebender/skills",
        project_skills_dir: r".firebender/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "kimi",
        display_name: "Kimi Code CLI",
        global_skills_dir: r"~/.kimi-code/skills",
        project_skills_dir: r".kimi-code/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "replit",
        display_name: "Replit",
        global_skills_dir: r"~/.config/agents/skills",
        project_skills_dir: r".config/agents/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        // Warp 与 cline 共用 `~/.agents/skills` 共享根（skills-manager 同源）。
        id: "warp",
        display_name: "Warp",
        global_skills_dir: r"~/.agents/skills",
        project_skills_dir: r".agents/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "augment",
        display_name: "Augment",
        global_skills_dir: r"~/.augment/skills",
        project_skills_dir: r".augment/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "bob",
        display_name: "IBM Bob",
        global_skills_dir: r"~/.bob/skills",
        project_skills_dir: r".bob/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "codebuddy",
        display_name: "CodeBuddy",
        global_skills_dir: r"~/.codebuddy/skills",
        project_skills_dir: r".codebuddy/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "command-code",
        display_name: "Command Code",
        global_skills_dir: r"~/.commandcode/skills",
        project_skills_dir: r".commandcode/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "cortex",
        display_name: "Cortex Code",
        global_skills_dir: r"~/.snowflake/cortex/skills",
        project_skills_dir: r".snowflake/cortex/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "crush",
        display_name: "Crush",
        global_skills_dir: r"~/.config/crush/skills",
        project_skills_dir: r".config/crush/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "iflow",
        display_name: "iFlow CLI",
        global_skills_dir: r"~/.iflow/skills",
        project_skills_dir: r".iflow/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "junie",
        display_name: "Junie",
        global_skills_dir: r"~/.junie/skills",
        project_skills_dir: r".junie/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "kiro",
        display_name: "Kiro CLI",
        global_skills_dir: r"~/.kiro/skills",
        project_skills_dir: r".kiro/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "kode",
        display_name: "Kode",
        global_skills_dir: r"~/.kode/skills",
        project_skills_dir: r".kode/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "mcpjam",
        display_name: "MCPJam",
        global_skills_dir: r"~/.mcpjam/skills",
        project_skills_dir: r".mcpjam/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "mistral-vibe",
        display_name: "Mistral Vibe",
        global_skills_dir: r"~/.vibe/skills",
        project_skills_dir: r".vibe/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "mux",
        display_name: "Mux",
        global_skills_dir: r"~/.mux/skills",
        project_skills_dir: r".mux/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "neovate",
        display_name: "Neovate",
        global_skills_dir: r"~/.neovate/skills",
        project_skills_dir: r".neovate/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        // oh-my-pi (OMP)：全局读 `~/.omp/agent/skills`，项目读 `<repo>/.omp/skills`
        // （非对称路径，skills-manager 同源注释）。
        id: "omp-agent",
        display_name: "OMP Agent",
        global_skills_dir: r"~/.omp/agent/skills",
        project_skills_dir: r".omp/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "pi",
        display_name: "Pi",
        global_skills_dir: r"~/.pi/agent/skills",
        project_skills_dir: r".pi/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "pochi",
        display_name: "Pochi",
        global_skills_dir: r"~/.pochi/skills",
        project_skills_dir: r".pochi/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "qwen-code",
        display_name: "Qwen Code",
        global_skills_dir: r"~/.qwen/skills",
        project_skills_dir: r".qwen/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        // TRAE CN：全局 `~/.trae-cn/skills`，项目级沿用 `.trae/skills`
        // （docs.trae.cn solo_skills，skills-manager 同源注释）。
        id: "trae-cn",
        display_name: "TRAE CN",
        global_skills_dir: r"~/.trae-cn/skills",
        project_skills_dir: r".trae/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "zencoder",
        display_name: "Zencoder",
        global_skills_dir: r"~/.zencoder/skills",
        project_skills_dir: r".zencoder/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "zcode",
        display_name: "ZCode",
        global_skills_dir: r"~/.zcode/skills",
        project_skills_dir: r".zcode/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "adal",
        display_name: "AdaL",
        global_skills_dir: r"~/.adal/skills",
        project_skills_dir: r".adal/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "qclaw",
        display_name: "QClaw",
        global_skills_dir: r"~/.qclaw/skills",
        project_skills_dir: r".qclaw/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "easyclaw",
        display_name: "EasyClaw",
        global_skills_dir: r"~/.easyclaw/skills",
        project_skills_dir: r".easyclaw/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "autoclaw",
        display_name: "AutoClaw",
        global_skills_dir: r"~/.openclaw-autoclaw/skills",
        project_skills_dir: r".openclaw-autoclaw/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "workbuddy-overseas",
        display_name: "WorkBuddy AI",
        global_skills_dir: r"~/.workbuddy-ai/skills",
        project_skills_dir: r".workbuddy-ai/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "deepseek-harness",
        display_name: "DeepSeek Harness",
        global_skills_dir: r"~/.dsh/skills",
        project_skills_dir: r".dsh/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        // GitLab Duo：全局 `~/.gitlab/duo/skills`，项目级共享 `.agents/skills`
        // （skills-manager 同源注释）。
        id: "gitlab-duo",
        display_name: "GitLab Duo",
        global_skills_dir: r"~/.gitlab/duo/skills",
        project_skills_dir: r".agents/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "minimax-code",
        display_name: "MiniMax Code",
        global_skills_dir: r"~/.minimax/skills",
        project_skills_dir: r".minimax/skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
];

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn ai_config_path(config_dir: &Path) -> PathBuf {
    config_dir.join(AI_CONFIG_FILE_NAME)
}

pub fn ai_config_corrupt_backup_path(config_dir: &Path) -> PathBuf {
    config_dir.join(AI_CONFIG_CORRUPT_BACKUP_NAME)
}

fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// Parse AI config text. Empty / whitespace / BOM-only → defaults (Ok).
/// Non-empty invalid JSON → Err with serde message (caller may recover).
pub fn parse_ai_config_text(text: &str) -> Result<AiConfigDocument, String> {
    let trimmed = strip_bom(text).trim();
    if trimmed.is_empty() {
        return Ok(AiConfigDocument::empty());
    }
    serde_json::from_str(trimmed).map_err(|err| format!("Failed to parse AI config JSON: {err}"))
}

/// Backup corrupt file once (skip if backup already exists), then remove/replace via rewrite.
fn backup_corrupt_once(config_dir: &Path, path: &Path) {
    let backup = ai_config_corrupt_backup_path(config_dir);
    if backup.exists() {
        return;
    }
    if let Err(err) = fs::copy(path, &backup) {
        eprintln!(
            "AIA-277: could not backup corrupt {}: {err}",
            path.display()
        );
    }
}

/// Atomic-ish write: temp file in same dir + rename (Windows: park existing then rename).
fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "AI config path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create AI config directory: {err}"))?;

    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| AI_CONFIG_FILE_NAME.to_string());
    let tmp = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    fs::write(&tmp, text).map_err(|err| {
        let _ = fs::remove_file(&tmp);
        format!("Failed to write AI config staging file: {err}")
    })?;

    if path.exists() {
        let parked = parent.join(format!(".{file_name}.{}.parked", std::process::id()));
        if let Err(err) = fs::rename(path, &parked) {
            let _ = fs::remove_file(&tmp);
            return Err(format!("Failed to park existing AI config: {err}"));
        }
        match fs::rename(&tmp, path) {
            Ok(()) => {
                let _ = fs::remove_file(&parked);
                Ok(())
            }
            Err(err) => {
                let _ = fs::rename(&parked, path);
                let _ = fs::remove_file(&tmp);
                Err(format!("Failed to commit AI config write: {err}"))
            }
        }
    } else {
        fs::rename(&tmp, path).map_err(|err| {
            let _ = fs::remove_file(&tmp);
            format!("Failed to commit AI config write: {err}")
        })
    }
}

pub use super::agent_paths::{expand_agent_path, path_exists_expanded};

pub fn refresh_discovered(target: &mut AgentTarget) {
    target.discovered = path_exists_expanded(&target.global_skills_dir)
        || path_exists_expanded(&target.global_mcp_path);
}

fn validate_id(raw: &str) -> Result<String, String> {
    let id = raw.trim().to_ascii_lowercase();
    if id.is_empty() {
        return Err(ERR_AGENT_ID_EMPTY.to_string());
    }
    if id.len() > MAX_ID_LEN {
        return Err(ERR_AGENT_ID_INVALID.to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ERR_AGENT_ID_INVALID.to_string());
    }
    if id.starts_with('-') || id.ends_with('-') {
        return Err(ERR_AGENT_ID_INVALID.to_string());
    }
    Ok(id)
}

/**
 * macOS/Unix 展示归一化：内置预设以 Windows 约定存储（`%USERPROFILE%\x\y`），
 * `expand_agent_path` 虽能在使用时展开，但编辑框会直接展示字面量，Mac 用户
 * 看到 `C:\Users\...` 式的无效路径。这里只在分隔符为 `/` 的宿主上，把
 * `%USERPROFILE%` 前缀写成可移植的 `~/`，并把 `\` 归一为 `/`。
 * `expand_agent_path` 原生支持 `~`，故发现/跳转/部署行为不变；Windows
 * 宿主（分隔符 `\`）原样返回，零行为变化。自定义路径不经此函数。
 */
fn display_path(raw: &str) -> String {
    if raw.is_empty() || crate::platform::current().path_separator() != '/' {
        return raw.to_string();
    }
    let mut out = raw.trim().replace('\\', "/");
    for token in ["%USERPROFILE%", "%APPDATA%", "%LOCALAPPDATA%"] {
        if out.to_ascii_uppercase().starts_with(token) {
            out = format!("~{}", &out[token.len()..]);
            break;
        }
    }
    out
}

fn builtin_target(def: &BuiltinDef, stamp: u64) -> AgentTarget {
    let (sessions, global_rules, project_rules) = builtin_extra_paths(def.id);
    let mut target = AgentTarget {
        id: def.id.to_string(),
        display_name: def.display_name.to_string(),
        kind: "builtin".to_string(),
        enabled: false,
        default_in_projects: false,
        discovered: false,
        global_skills_dir: display_path(def.global_skills_dir),
        project_skills_dir: display_path(def.project_skills_dir),
        global_mcp_path: display_path(def.global_mcp_path),
        project_mcp_path: display_path(def.project_mcp_path),
        global_sessions_dir: display_path(sessions),
        global_rules_path: display_path(global_rules),
        project_rules_dir: display_path(project_rules),
        mcp_usage_prompt: String::new(),
        created_at: stamp,
        updated_at: stamp,
    };
    refresh_discovered(&mut target);
    target
}

/// Ensure all builtins exist in the document; refresh discovery flags.
pub fn sync_builtins_and_discover(document: &AiConfigDocument) -> AiConfigDocument {
    let stamp = now_millis();
    let mut next = document.clone();
    let migrate_defaults = next.schema_version < 2;
    // ADR 0028：v2→v3 一次性迁移——内置条目按补充预设表种入 sessions/rules 槽
    // （空才种；用户已填/已清空的值不覆盖）。
    let migrate_extra_paths = next.schema_version < 3;
    // v3→v4：macOS/Unix 上把内置条目的 Windows 路径字面量
    //（`%USERPROFILE%\...`）归一化成可移植的 `~/...`，只动仍含 token 的
    // 预设值；用户自定义过的具体路径不改写。
    let migrate_display_paths = next.schema_version < 4;
    next.schema_version = AI_CONFIG_SCHEMA_VERSION;
    next.skill_deploy_mode = normalize_skill_deploy_mode(&next.skill_deploy_mode);

    for def in BUILTIN_DEFS {
        if let Some(existing) = next.agent_targets.iter_mut().find(|t| t.id == def.id) {
            // Keep user overrides for paths / prompt / enabled; refresh discovery only.
            if existing.kind != "custom" {
                existing.kind = "builtin".to_string();
            }
            if migrate_extra_paths {
                let (sessions, global_rules, project_rules) = builtin_extra_paths(def.id);
                if existing.global_sessions_dir.trim().is_empty() && !sessions.is_empty() {
                    existing.global_sessions_dir = sessions.to_string();
                }
                if existing.global_rules_path.trim().is_empty() && !global_rules.is_empty() {
                    existing.global_rules_path = global_rules.to_string();
                }
                if existing.project_rules_dir.trim().is_empty() && !project_rules.is_empty() {
                    existing.project_rules_dir = project_rules.to_string();
                }
            }
            if migrate_display_paths {
                // Only rewrite fields still holding the untouched preset value.
                // A user-edited path never equals the raw Windows convention.
                for (slot, raw) in [
                    (&mut existing.global_skills_dir, def.global_skills_dir),
                    (&mut existing.project_skills_dir, def.project_skills_dir),
                    (&mut existing.global_mcp_path, def.global_mcp_path),
                    (&mut existing.project_mcp_path, def.project_mcp_path),
                ] {
                    if *slot == raw {
                        *slot = display_path(raw);
                    }
                }
                let (sessions, global_rules, project_rules) = builtin_extra_paths(def.id);
                for (slot, raw) in [
                    (&mut existing.global_sessions_dir, sessions),
                    (&mut existing.global_rules_path, global_rules),
                    (&mut existing.project_rules_dir, project_rules),
                ] {
                    if *slot == raw {
                        *slot = display_path(raw);
                    }
                }
            }
            refresh_discovered(existing);
        } else {
            next.agent_targets.push(builtin_target(def, stamp));
        }
    }

    for target in &mut next.agent_targets {
        if migrate_defaults {
            // ADR 0010：首次出现时 defaultInProjects = enabled。
            target.default_in_projects = target.enabled;
        }
        refresh_discovered(target);
    }

    next.agent_targets.sort_by(|a, b| {
        a.display_name
            .to_ascii_lowercase()
            .cmp(&b.display_name.to_ascii_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    next
}

pub fn load_from_path(config_dir: &Path) -> Result<AiConfigDocument, String> {
    let path = ai_config_path(config_dir);
    let mut force_rewrite = false;
    let document = if !path.exists() {
        AiConfigDocument::empty()
    } else {
        let text =
            fs::read_to_string(&path).map_err(|err| format!("Failed to read AI config: {err}"))?;
        let stripped = strip_bom(&text);
        // Empty / whitespace / BOM-only: silent recover (AIA-277 — not a user-facing failure).
        if stripped.trim().is_empty() {
            force_rewrite = true;
            AiConfigDocument::empty()
        } else {
            match parse_ai_config_text(&text) {
                Ok(doc) => doc,
                Err(err) => {
                    // Non-empty corrupt JSON: soft-recover to defaults; keep one backup.
                    eprintln!(
                        "AIA-277: recovering corrupt AI config at {}: {err}",
                        path.display()
                    );
                    backup_corrupt_once(config_dir, &path);
                    force_rewrite = true;
                    AiConfigDocument::empty()
                }
            }
        }
    };
    let synced = crate::ai::mcp::sync_seed_catalog(&crate::ai::providers::sync_presets(
        &sync_builtins_and_discover(&document),
    ));
    if force_rewrite || synced != document {
        save_to_path(config_dir, &synced)?;
    }
    Ok(synced)
}

pub fn save_to_path(config_dir: &Path, document: &AiConfigDocument) -> Result<(), String> {
    let path = ai_config_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize AI config: {err}"))?;
    write_text_atomic(&path, &text)
}

pub fn set_enabled(
    document: &AiConfigDocument,
    id: &str,
    enabled: bool,
) -> Result<AiConfigDocument, String> {
    let mut next = document.clone();
    let target = next
        .agent_targets
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| ERR_AGENT_UNKNOWN.to_string())?;
    target.enabled = enabled;
    target.updated_at = now_millis();
    refresh_discovered(target);
    Ok(next)
}

pub fn set_default_in_projects(
    document: &AiConfigDocument,
    id: &str,
    default_in_projects: bool,
) -> Result<AiConfigDocument, String> {
    let mut next = document.clone();
    let target = next
        .agent_targets
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| ERR_AGENT_UNKNOWN.to_string())?;
    target.default_in_projects = default_in_projects;
    target.updated_at = now_millis();
    refresh_discovered(target);
    Ok(next)
}

pub fn set_skill_deploy_mode(
    document: &AiConfigDocument,
    mode: &str,
) -> Result<AiConfigDocument, String> {
    let mut next = document.clone();
    next.skill_deploy_mode = normalize_skill_deploy_mode(mode);
    Ok(next)
}

pub fn update_target(
    document: &AiConfigDocument,
    id: &str,
    patch: AgentTargetPatch,
) -> Result<AiConfigDocument, String> {
    let mut next = document.clone();
    let target = next
        .agent_targets
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| ERR_AGENT_UNKNOWN.to_string())?;

    if let Some(name) = patch.display_name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(ERR_AGENT_NAME_EMPTY.to_string());
        }
        target.display_name = trimmed.to_string();
    }
    if let Some(enabled) = patch.enabled {
        target.enabled = enabled;
    }
    if let Some(default_in_projects) = patch.default_in_projects {
        target.default_in_projects = default_in_projects;
    }
    if let Some(v) = patch.global_skills_dir {
        target.global_skills_dir = v.trim().to_string();
    }
    if let Some(v) = patch.project_skills_dir {
        target.project_skills_dir = v.trim().to_string();
    }
    if let Some(v) = patch.global_mcp_path {
        target.global_mcp_path = v.trim().to_string();
    }
    if let Some(v) = patch.project_mcp_path {
        target.project_mcp_path = v.trim().to_string();
    }
    if let Some(v) = patch.global_sessions_dir {
        target.global_sessions_dir = v.trim().to_string();
    }
    if let Some(v) = patch.global_rules_path {
        target.global_rules_path = v.trim().to_string();
    }
    if let Some(v) = patch.project_rules_dir {
        target.project_rules_dir = v.trim().to_string();
    }
    if let Some(v) = patch.mcp_usage_prompt {
        // Placeholder field — store as-is; never inject (A2).
        target.mcp_usage_prompt = v;
    }
    target.updated_at = now_millis();
    refresh_discovered(target);
    Ok(next)
}

pub fn upsert_custom(
    document: &AiConfigDocument,
    id: Option<String>,
    display_name: String,
    global_skills_dir: String,
    project_skills_dir: String,
    global_mcp_path: String,
    project_mcp_path: String,
    global_sessions_dir: String,
    global_rules_path: String,
    project_rules_dir: String,
    mcp_usage_prompt: String,
) -> Result<AiConfigDocument, String> {
    let name = display_name.trim();
    if name.is_empty() {
        return Err(ERR_AGENT_NAME_EMPTY.to_string());
    }
    let stamp = now_millis();
    let resolved_id = if let Some(raw) = id {
        validate_id(&raw)?
    } else {
        let slug = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let collapsed = slug
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        validate_id(&format!("custom-{collapsed}"))?
    };

    let mut next = document.clone();
    if let Some(existing) = next.agent_targets.iter_mut().find(|t| t.id == resolved_id) {
        if existing.kind == "builtin" {
            return Err(ERR_AGENT_ID_CONFLICT.to_string());
        }
        existing.display_name = name.to_string();
        existing.global_skills_dir = global_skills_dir.trim().to_string();
        existing.project_skills_dir = project_skills_dir.trim().to_string();
        existing.global_mcp_path = global_mcp_path.trim().to_string();
        existing.project_mcp_path = project_mcp_path.trim().to_string();
        existing.global_sessions_dir = global_sessions_dir.trim().to_string();
        existing.global_rules_path = global_rules_path.trim().to_string();
        existing.project_rules_dir = project_rules_dir.trim().to_string();
        existing.mcp_usage_prompt = mcp_usage_prompt;
        existing.updated_at = stamp;
        refresh_discovered(existing);
        return Ok(next);
    }

    if BUILTIN_DEFS.iter().any(|d| d.id == resolved_id) {
        return Err(ERR_AGENT_ID_CONFLICT.to_string());
    }

    let mut target = AgentTarget {
        id: resolved_id,
        display_name: name.to_string(),
        kind: "custom".to_string(),
        enabled: false,
        default_in_projects: false,
        discovered: false,
        global_skills_dir: global_skills_dir.trim().to_string(),
        project_skills_dir: project_skills_dir.trim().to_string(),
        global_mcp_path: global_mcp_path.trim().to_string(),
        project_mcp_path: project_mcp_path.trim().to_string(),
        global_sessions_dir: global_sessions_dir.trim().to_string(),
        global_rules_path: global_rules_path.trim().to_string(),
        project_rules_dir: project_rules_dir.trim().to_string(),
        mcp_usage_prompt,
        created_at: stamp,
        updated_at: stamp,
    };
    refresh_discovered(&mut target);
    next.agent_targets.push(target);
    next.agent_targets.sort_by(|a, b| {
        a.display_name
            .to_ascii_lowercase()
            .cmp(&b.display_name.to_ascii_lowercase())
    });
    Ok(next)
}

pub fn remove_custom(document: &AiConfigDocument, id: &str) -> Result<AiConfigDocument, String> {
    let Some(target) = document.agent_targets.iter().find(|t| t.id == id) else {
        return Err(ERR_AGENT_UNKNOWN.to_string());
    };
    if target.kind != "custom" {
        return Err(ERR_AGENT_BUILTIN_REMOVE.to_string());
    }
    let mut next = document.clone();
    next.agent_targets.retain(|t| t.id != id);
    Ok(next)
}

/// ADR 0028：slot 名 → Agent Target 全局路径（已展开 token），供 `ai_agent_target_open_path`。
/// 返回 `Err` 时携带错误码（`AI_AGENT_SLOT_UNKNOWN` / `AI_AGENT_PATH_EMPTY`）。
pub fn resolve_global_slot_path(target: &AgentTarget, slot: &str) -> Result<String, String> {
    let raw = match slot {
        SLOT_GLOBAL_SKILLS_DIR => target.global_skills_dir.as_str(),
        SLOT_GLOBAL_SESSIONS_DIR => target.global_sessions_dir.as_str(),
        SLOT_GLOBAL_RULES_PATH => target.global_rules_path.as_str(),
        SLOT_GLOBAL_MCP_PATH => target.global_mcp_path.as_str(),
        _ => return Err(ERR_AGENT_SLOT_UNKNOWN.to_string()),
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ERR_AGENT_PATH_EMPTY.to_string());
    }
    Ok(expand_agent_path(trimmed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("afm-ai-agents-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn seeds_builtins_and_persists() {
        let dir = temp_dir("seed");
        let doc = load_from_path(&dir).unwrap();
        assert!(doc.agent_targets.len() >= BUILTIN_DEFS.len());
        assert!(doc.agent_targets.iter().any(|t| t.id == "cursor"));
        assert!(ai_config_path(&dir).is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn seeds_expanded_builtin_registry_from_skills_manager() {
        // 2026-11 批量收录（skills-manager v1.44 注册表）：加载即补种，
        // 无需 schema 迁移；内置 id 不得重复。
        let dir = temp_dir("expanded");
        let doc = load_from_path(&dir).unwrap();
        for id in [
            "codex",
            "github-copilot",
            "gemini-cli",
            "antigravity",
            "windsurf",
            "kilo-code",
            "omp-agent",
            "trae-cn",
            "gitlab-duo",
            "minimax-code",
            "deepseek-harness",
        ] {
            assert!(
                doc.agent_targets.iter().any(|t| t.id == id),
                "missing builtin {id}"
            );
        }
        let mut ids: Vec<&str> = BUILTIN_DEFS.iter().map(|d| d.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "builtin ids must be unique");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn enable_and_update_prompt_placeholder() {
        let dir = temp_dir("enable");
        let doc = load_from_path(&dir).unwrap();
        let enabled = set_enabled(&doc, "cursor", true).unwrap();
        let cursor = enabled
            .agent_targets
            .iter()
            .find(|t| t.id == "cursor")
            .unwrap();
        assert!(cursor.enabled);

        let patched = update_target(
            &enabled,
            "cursor",
            AgentTargetPatch {
                display_name: None,
                enabled: None,
                default_in_projects: None,
                global_skills_dir: None,
                project_skills_dir: None,
                global_mcp_path: None,
                project_mcp_path: None,
                global_sessions_dir: None,
                global_rules_path: None,
                project_rules_dir: None,
                mcp_usage_prompt: Some("PLACEHOLDER — do not inject".into()),
            },
        )
        .unwrap();
        let cursor = patched
            .agent_targets
            .iter()
            .find(|t| t.id == "cursor")
            .unwrap();
        assert_eq!(cursor.mcp_usage_prompt, "PLACEHOLDER — do not inject");
        // Field is stored only; no injection helper exists in this module.
        assert!(!cursor.mcp_usage_prompt.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn custom_upsert_and_remove() {
        let dir = temp_dir("custom");
        let doc = load_from_path(&dir).unwrap();
        let with = upsert_custom(
            &doc,
            Some("my-agent".into()),
            "My Agent".into(),
            r"%USERPROFILE%\.my-agent\skills".into(),
            r".my-agent\skills".into(),
            r"%USERPROFILE%\.my-agent\mcp.json".into(),
            r".my-agent\mcp.json".into(),
            r"%USERPROFILE%\.my-agent\sessions".into(),
            r"%USERPROFILE%\.my-agent\RULES.md".into(),
            r".my-agent\rules".into(),
            String::new(),
        )
        .unwrap();
        assert!(with
            .agent_targets
            .iter()
            .any(|t| t.id == "my-agent" && t.kind == "custom"));
        let mine = with
            .agent_targets
            .iter()
            .find(|t| t.id == "my-agent")
            .unwrap();
        assert_eq!(mine.global_sessions_dir, r"%USERPROFILE%\.my-agent\sessions");
        assert_eq!(mine.global_rules_path, r"%USERPROFILE%\.my-agent\RULES.md");
        assert_eq!(mine.project_rules_dir, r".my-agent\rules");

        let removed = remove_custom(&with, "my-agent").unwrap();
        assert!(!removed.agent_targets.iter().any(|t| t.id == "my-agent"));

        assert_eq!(
            remove_custom(&removed, "cursor").unwrap_err(),
            ERR_AGENT_BUILTIN_REMOVE
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_custom_id_colliding_with_builtin() {
        let dir = temp_dir("conflict");
        let doc = load_from_path(&dir).unwrap();
        let err = upsert_custom(
            &doc,
            Some("cursor".into()),
            "Fake Cursor".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap_err();
        assert_eq!(err, ERR_AGENT_ID_CONFLICT);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_userprofile_token() {
        let expanded = expand_agent_path(r"%USERPROFILE%\.cursor\skills");
        assert!(!expanded.contains('%'));
        let sep = crate::platform::current().path_separator();
        assert!(expanded.ends_with(&format!("{sep}.cursor{sep}skills")));
    }

    #[test]
    fn every_builtin_path_expands_without_tokens_or_foreign_separators() {
        // macOS port regression: builtins are stored in Windows convention
        // (`%USERPROFILE%\.cursor\skills`). After expansion no `%TOKEN%` and no
        // foreign separator may remain, otherwise discovery/deploy silently fail.
        let foreign = if crate::platform::current().path_separator() == '/' {
            '\\'
        } else {
            '/'
        };
        for def in BUILTIN_DEFS {
            for raw in [
                def.global_skills_dir,
                def.project_skills_dir,
                def.global_mcp_path,
                def.project_mcp_path,
            ] {
                let expanded = super::expand_agent_path(raw);
                assert!(!expanded.contains('%'), "token left in {raw} -> {expanded}");
                assert!(
                    !expanded.contains(foreign),
                    "foreign separator in {raw} -> {expanded}"
                );
            }
        }
    }

    #[test]
    fn every_builtin_extra_path_expands_without_tokens_or_foreign_separators() {
        // ADR 0028：补充预设表同样必须无残留 token / 异平台分隔符。
        let foreign = if crate::platform::current().path_separator() == '/' {
            '\\'
        } else {
            '/'
        };
        for (_, sessions, global_rules, project_rules) in BUILTIN_EXTRA_PATHS {
            for raw in [*sessions, *global_rules, *project_rules] {
                if raw.is_empty() {
                    continue;
                }
                let expanded = super::expand_agent_path(raw);
                assert!(
                    !expanded.contains('%') && !expanded.contains('~'),
                    "token left in {raw} -> {expanded}"
                );
                assert!(
                    !expanded.contains(foreign),
                    "foreign separator in {raw} -> {expanded}"
                );
            }
        }
    }

    #[test]
    fn display_path_normalizes_windows_presets_on_unix_only() {
        let is_unix = crate::platform::current().path_separator() == '/';
        let result = display_path(r"%USERPROFILE%\.cursor\skills");
        if is_unix {
            assert_eq!(result, "~/.cursor/skills");
            assert!(!result.contains('%') && !result.contains('\\'));
            assert_eq!(display_path(r".claude\skills"), ".claude/skills");
        } else {
            assert_eq!(result, r"%USERPROFILE%\.cursor\skills");
        }
        // 归一化后的值仍可被展开器解析（发现 / 跳转 / 部署不受影响）。
        let expanded = expand_agent_path(&result);
        assert!(!expanded.contains('%') && !expanded.contains('~'));
    }

    #[test]
    fn migrates_v2_seeds_extra_paths_once_and_keeps_user_values() {
        // ADR 0028：v2 配置加载即补种 sessions/rules 槽；用户已有值不覆盖；
        // 迁移一次性完成——之后用户清空也不会被回种。
        let dir = temp_dir("v2-migrate");
        let path = ai_config_path(&dir);
        let v2_text = serde_json::json!({
            "schemaVersion": 2,
            "agentTargets": [
                {
                    "id": "claude-code",
                    "displayName": "Claude Code",
                    "kind": "builtin",
                    "enabled": true,
                    "discovered": true,
                    "globalSkillsDir": "%USERPROFILE%\\.claude\\skills",
                    "projectSkillsDir": ".claude\\skills",
                    "globalMcpPath": "%USERPROFILE%\\.claude.json",
                    "projectMcpPath": ".mcp.json",
                    "mcpUsagePrompt": "",
                    "createdAt": 1,
                    "updatedAt": 1
                }
            ],
            "skillDeployMode": "junction"
        })
        .to_string();
        fs::write(&path, v2_text).unwrap();
        let doc = load_from_path(&dir).unwrap();
        assert_eq!(doc.schema_version, AI_CONFIG_SCHEMA_VERSION);
        let claude = doc.agent_targets.iter().find(|t| t.id == "claude-code").unwrap();
        // v3→v4 在 Unix 上把未改动的预设归一化成 `~/...`；Windows 原样保留。
        assert_eq!(
            claude.global_sessions_dir,
            display_path(r"%USERPROFILE%\.claude\projects")
        );
        assert_eq!(
            claude.global_rules_path,
            display_path(r"%USERPROFILE%\.claude\CLAUDE.md")
        );
        assert_eq!(claude.project_rules_dir, display_path(r".claude"));

        // 用户改写后重载：保留用户值，不回种。
        let patched = update_target(
            &doc,
            "claude-code",
            AgentTargetPatch {
                display_name: None,
                enabled: None,
                default_in_projects: None,
                global_skills_dir: None,
                project_skills_dir: None,
                global_mcp_path: None,
                project_mcp_path: None,
                global_sessions_dir: Some("D:\\custom-sessions".into()),
                global_rules_path: Some(String::new()),
                project_rules_dir: None,
                mcp_usage_prompt: None,
            },
        )
        .unwrap();
        save_to_path(&dir, &patched).unwrap();
        let reloaded = load_from_path(&dir).unwrap();
        let claude = reloaded.agent_targets.iter().find(|t| t.id == "claude-code").unwrap();
        assert_eq!(claude.global_sessions_dir, r"D:\custom-sessions");
        assert_eq!(claude.global_rules_path, "", "user-cleared value must not be re-seeded");
        assert_eq!(claude.project_rules_dir, display_path(r".claude"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolves_global_slot_paths_with_expansion_and_errors() {
        let dir = temp_dir("slot");
        let doc = load_from_path(&dir).unwrap();
        let claude = doc.agent_targets.iter().find(|t| t.id == "claude-code").unwrap();
        let sessions = resolve_global_slot_path(claude, SLOT_GLOBAL_SESSIONS_DIR).unwrap();
        assert!(!sessions.contains('%'), "token must expand: {sessions}");
        assert!(sessions.ends_with(".claude\\projects") || sessions.ends_with(".claude/projects"));
        let rules = resolve_global_slot_path(claude, SLOT_GLOBAL_RULES_PATH).unwrap();
        assert!(!rules.contains('%'), "token must expand: {rules}");
        assert!(rules.contains("CLAUDE.md"));
        // 无会话预设的 Agent → AI_AGENT_PATH_EMPTY。
        let cline = doc.agent_targets.iter().find(|t| t.id == "cline").unwrap();
        assert_eq!(
            resolve_global_slot_path(cline, SLOT_GLOBAL_SESSIONS_DIR).unwrap_err(),
            ERR_AGENT_PATH_EMPTY
        );
        // 未知槽位 → AI_AGENT_SLOT_UNKNOWN。
        assert_eq!(
            resolve_global_slot_path(claude, "nope").unwrap_err(),
            ERR_AGENT_SLOT_UNKNOWN
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_empty_whitespace_bom_yields_default() {
        assert_eq!(parse_ai_config_text("").unwrap(), AiConfigDocument::empty());
        assert_eq!(
            parse_ai_config_text("   \n\t  ").unwrap(),
            AiConfigDocument::empty()
        );
        assert_eq!(
            parse_ai_config_text("\u{feff}").unwrap(),
            AiConfigDocument::empty()
        );
        assert_eq!(
            parse_ai_config_text("\u{feff}  \r\n").unwrap(),
            AiConfigDocument::empty()
        );
    }

    #[test]
    fn parse_corrupt_json_errors() {
        let err = parse_ai_config_text("{not-json").unwrap_err();
        assert!(err.contains("Failed to parse AI config JSON"));
    }

    #[test]
    fn load_empty_file_recovers_defaults_without_error() {
        let dir = temp_dir("empty-file");
        let path = ai_config_path(&dir);
        fs::write(&path, "").unwrap();
        let doc = load_from_path(&dir).expect("empty file must not hard-fail");
        assert!(doc.agent_targets.len() >= BUILTIN_DEFS.len());
        assert!(!doc.ai_providers.is_empty(), "presets should seed");
        let rewritten = fs::read_to_string(&path).unwrap();
        assert!(
            rewritten.trim_start().starts_with('{'),
            "empty file should be rewritten as valid JSON"
        );
        assert!(!ai_config_corrupt_backup_path(&dir).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_whitespace_and_bom_file_recovers_defaults() {
        let dir = temp_dir("bom-ws");
        let path = ai_config_path(&dir);
        fs::write(&path, "\u{feff} \n\t").unwrap();
        let doc = load_from_path(&dir).expect("BOM/whitespace must not hard-fail");
        assert!(doc.schema_version >= AI_CONFIG_SCHEMA_VERSION);
        assert!(!ai_config_corrupt_backup_path(&dir).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_corrupt_file_backs_up_once_and_rewrites_defaults() {
        let dir = temp_dir("corrupt");
        let path = ai_config_path(&dir);
        let junk = "{not valid json!!!";
        fs::write(&path, junk).unwrap();
        let doc = load_from_path(&dir).expect("corrupt JSON must recover");
        assert!(doc.agent_targets.len() >= BUILTIN_DEFS.len());

        let backup = ai_config_corrupt_backup_path(&dir);
        assert!(backup.is_file());
        assert_eq!(fs::read_to_string(&backup).unwrap(), junk);

        let rewritten = fs::read_to_string(&path).unwrap();
        assert!(serde_json::from_str::<AiConfigDocument>(&rewritten).is_ok());

        // Second load must not overwrite the first backup.
        fs::write(&path, "%%%still-bad%%%").unwrap();
        let _ = load_from_path(&dir).unwrap();
        assert_eq!(fs::read_to_string(&backup).unwrap(), junk);
        let _ = fs::remove_dir_all(&dir);
    }
}
