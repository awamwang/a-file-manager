// SPDX-License-Identifier: GPL-3.0-or-later

use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const DEFAULT_WORK_GROUP_ID: &str = "default-work-group";

/// AIA-245: accept negative / null / float OpenState indices from the webview
/// instead of failing `usize` deserialize (which blocked silent WG switch).
fn deserialize_open_state_index<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    struct IndexVisitor;

    impl<'de> Visitor<'de> for IndexVisitor {
        type Value = usize;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a non-negative OpenState index")
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(0)
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(0)
        }

        fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
            Ok(if value < 0 { 0 } else { value as usize })
        }

        fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
            Ok(value as usize)
        }

        fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
            if !value.is_finite() || value < 0.0 {
                return Ok(0);
            }
            Ok(value.trunc() as usize)
        }
    }

    deserializer.deserialize_any(IndexVisitor)
}

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
    #[serde(deserialize_with = "deserialize_open_state_index")]
    pub current_tab_group_index: usize,
    #[serde(deserialize_with = "deserialize_open_state_index")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_state_indices_accept_negative_and_null() {
        let raw = r#"{
            "tabGroups": [[{
                "id": "t1",
                "name": "keep",
                "path": "C:/keep",
                "type": "directory"
            }]],
            "currentTabGroupIndex": -1,
            "currentTabIndex": null
        }"#;
        let state: WorkGroupOpenState = serde_json::from_str(raw).expect("deserialize");
        assert_eq!(state.current_tab_group_index, 0);
        assert_eq!(state.current_tab_index, 0);
    }
}
