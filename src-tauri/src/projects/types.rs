// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

pub const TASK_STATUSES: [&str; 4] = ["backlog", "doing", "review", "done"];

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
    /// Locked: open-all never mutates Work Groups.
    pub work_group_mutated: bool,
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
