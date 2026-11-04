// SPDX-License-Identifier: GPL-3.0-or-later

use crate::work_groups::types::SilentSwitchResult;
use serde::{Deserialize, Serialize};

/// v2: Project may bind a non-default Work Group (`workGroupId`) for session authority.
pub const SCHEMA_VERSION: u32 = 2;

pub const TASK_STATUSES: [&str; 4] = ["backlog", "doing", "review", "done"];

/// Stable IPC error codes (UI maps via i18n).
pub const ERR_DEFAULT_WORK_GROUP_FORBIDDEN: &str = "PROJECT_DEFAULT_WORK_GROUP_FORBIDDEN";
pub const ERR_UNKNOWN_WORK_GROUP: &str = "PROJECT_UNKNOWN_WORK_GROUP";
pub const ERR_PROJECT_UNBOUND: &str = "PROJECT_UNBOUND";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLink {
    pub id: String,
    pub path: String,
    pub label: String,
    pub kind: String,
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTask {
    pub id: String,
    pub title: String,
    pub body: String,
    pub status_id: String,
    pub sort_key: i64,
    pub workspace_link_id: Option<String>,
    pub outcome_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserProject {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Bound Work Group id for session (members + openState). Never the default WG.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_group_id: Option<String>,
    pub workspace_links: Vec<WorkspaceLink>,
    pub tasks: Vec<ProjectTask>,
    pub context_notes: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserProjects {
    pub schema_version: u32,
    pub projects: Vec<UserProject>,
    pub active_project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenAllLinksResult {
    pub project_id: String,
    pub paths: Vec<String>,
    /// Open-all still does not rewrite Work Group hard config / binding.
    pub work_group_mutated: bool,
}

/// Activate Project = set active + optional silent-switch of bound WG into the shell strip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivateProjectResult {
    pub projects: UserProjects,
    pub silent_switch: Option<SilentSwitchResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub status_id: Option<String>,
    pub sort_key: Option<i64>,
    pub workspace_link_id: Option<Option<String>>,
    pub outcome_url: Option<Option<String>>,
}
