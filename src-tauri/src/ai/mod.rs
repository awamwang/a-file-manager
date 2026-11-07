// SPDX-License-Identifier: GPL-3.0-or-later
//! AI Add-on layer (AIA-214 / A1).
//!
//! **Boundary (ADR 0007):** Workspace / Projects Root / WG / Navigator ∈ Core FM.
//! This module owns the AI Add-on surface: main menu shell, and later Agent Target,
//! public Skills/MCP, Providers, Harness/SDD init (A2+). Core must not grow those.
//!
//! A1 only exposes a shell status + fixed placeholder section ids. No Agent registry,
//! no Skill/MCP deploy, no LLM Providers yet.

use serde::Serialize;

/// Stable section ids for the AI main menu shell (UI + future IPC).
pub const AI_MENU_SECTION_IDS: &[&str] = &["agents", "mcp", "skills", "providers"];

pub const AI_ADDON_SHELL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiMenuSectionStatus {
    pub id: String,
    /// False until the slice that owns this section lands (A2–A7).
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiAddonStatus {
    /// Always `"ai-addon"` — distinguishes from Core FM modules.
    pub layer: String,
    pub shell_version: u32,
    pub sections: Vec<AiMenuSectionStatus>,
}

/// Shell probe for the AI main menu (placeholder sections only).
#[tauri::command]
pub fn ai_addon_status() -> AiAddonStatus {
    AiAddonStatus {
        layer: "ai-addon".to_string(),
        shell_version: AI_ADDON_SHELL_VERSION,
        sections: AI_MENU_SECTION_IDS
            .iter()
            .map(|id| AiMenuSectionStatus {
                id: (*id).to_string(),
                ready: false,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_exposes_four_placeholder_sections() {
        let status = ai_addon_status();
        assert_eq!(status.layer, "ai-addon");
        assert_eq!(status.shell_version, AI_ADDON_SHELL_VERSION);
        assert_eq!(
            status
                .sections
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            AI_MENU_SECTION_IDS
        );
        assert!(status.sections.iter().all(|s| !s.ready));
    }
}
