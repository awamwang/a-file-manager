// SPDX-License-Identifier: GPL-3.0-or-later
//! 共享时间戳 / id / 路径标签 / WG 绑定校验（Workspace 与 legacy migrate 共用）。

use super::types::{ERR_DEFAULT_WORK_GROUP_FORBIDDEN, ERR_UNKNOWN_WORK_GROUP};
use crate::work_groups::types::{UserWorkGroups, DEFAULT_WORK_GROUP_ID};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn next_id(prefix: &str) -> String {
    format!("{prefix}-{}", now_millis())
}

pub fn basename_label(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

/// Reject default WG and unknown ids. `work_groups` is the authority for existence / isDefault.
pub fn validate_bindable_work_group(
    work_groups: &UserWorkGroups,
    work_group_id: &str,
) -> Result<(), String> {
    if work_group_id == DEFAULT_WORK_GROUP_ID {
        return Err(ERR_DEFAULT_WORK_GROUP_FORBIDDEN.to_string());
    }
    let group = work_groups
        .work_groups
        .iter()
        .find(|group| group.id == work_group_id)
        .ok_or_else(|| ERR_UNKNOWN_WORK_GROUP.to_string())?;
    if group.is_default {
        return Err(ERR_DEFAULT_WORK_GROUP_FORBIDDEN.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_groups::sync::{create_default_work_group, create_work_group_in_list};
    use crate::work_groups::types::UserWorkGroups;

    #[test]
    fn basename_handles_windows_and_trailing_slash() {
        assert_eq!(basename_label(r"D:\Work\demo"), "demo");
        assert_eq!(basename_label("D:/Work/demo/"), "demo");
    }

    #[test]
    fn validate_rejects_default_wg() {
        let default = create_default_work_group(None);
        let (list, created) = create_work_group_in_list(&[default], Some("Focus".into()));
        let wgs = UserWorkGroups {
            work_groups: list,
            active_work_group_id: DEFAULT_WORK_GROUP_ID.to_string(),
        };
        assert_eq!(
            validate_bindable_work_group(&wgs, DEFAULT_WORK_GROUP_ID).unwrap_err(),
            ERR_DEFAULT_WORK_GROUP_FORBIDDEN
        );
        assert!(validate_bindable_work_group(&wgs, &created.id).is_ok());
    }
}
