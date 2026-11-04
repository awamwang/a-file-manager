// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportMode {
    /// Overwrite AFM work-groups.json + projects.json with mapped Sigma data.
    Replace,
    /// Keep existing AFM ids; only append missing Work Groups / Projects; fill empty openState.
    Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LossyField {
    pub surface: String,
    pub field: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlan {
    pub mode: ImportMode,
    pub work_group_count: usize,
    pub project_count: usize,
    pub navigator_tab_group_count: usize,
    pub lossy_fields: Vec<LossyField>,
    pub notes: Vec<String>,
}
