// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};

pub const DEFAULT_WORK_GROUP_ID: &str = "default-work-group";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkGroupOpenStateTab {
    pub id: String,
    pub name: String,
    pub path: String,
    /// "directory" | "file" | "search"
    #[serde(rename = "type")]
    pub tab_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkGroupOpenState {
    pub tab_groups: Vec<Vec<WorkGroupOpenStateTab>>,
    pub current_tab_group_index: usize,
    pub current_tab_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkGroup {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub member_paths: Vec<String>,
    pub open_state: Option<WorkGroupOpenState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserWorkGroups {
    pub work_groups: Vec<WorkGroup>,
    pub active_work_group_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkGroupSoftSyncNotice {
    pub kind: String,
    pub work_group_id: String,
    pub opened_member_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SilentSwitchResult {
    pub document: UserWorkGroups,
    pub open_state_to_restore: Option<WorkGroupOpenState>,
    pub require_save_prompt: bool,
    pub relative_member_paths_to_open: Vec<String>,
    pub notice: Option<WorkGroupSoftSyncNotice>,
}
