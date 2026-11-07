// SPDX-License-Identifier: GPL-3.0-or-later

use super::paths::{is_exact_member_path, unique_member_paths};
use super::types::{
    SilentSwitchResult, UserWorkGroups, WorkGroup, WorkGroupOpenState, WorkGroupOpenStateTab,
    WorkGroupSoftSyncNotice, DEFAULT_WORK_GROUP_ID,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn next_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{prefix}-{millis}")
}

fn display_name(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

pub fn create_empty_open_state() -> WorkGroupOpenState {
    WorkGroupOpenState {
        tab_groups: Vec::new(),
        current_tab_group_index: 0,
        current_tab_index: 0,
    }
}

pub fn create_directory_open_tab(path: &str) -> WorkGroupOpenStateTab {
    WorkGroupOpenStateTab {
        id: next_id("tab"),
        name: display_name(path),
        path: path.to_string(),
        tab_type: "directory".to_string(),
    }
}

pub fn create_default_work_group(open_state: Option<WorkGroupOpenState>) -> WorkGroup {
    WorkGroup {
        id: DEFAULT_WORK_GROUP_ID.to_string(),
        name: "Default".to_string(),
        is_default: true,
        member_paths: Vec::new(),
        open_state,
    }
}

pub fn default_document() -> UserWorkGroups {
    UserWorkGroups {
        work_groups: vec![create_default_work_group(None)],
        active_work_group_id: DEFAULT_WORK_GROUP_ID.to_string(),
    }
}

pub fn open_state_has_exact_path(open_state: &WorkGroupOpenState, path: &str) -> bool {
    for group in &open_state.tab_groups {
        for tab in group {
            if is_exact_member_path(&tab.path, &[path.to_string()]) {
                return true;
            }
        }
    }
    false
}

/// Hard → runtime: ensure each member path has a directory tab (never closes extras).
pub fn ensure_member_tabs_in_open_state(
    open_state: Option<&WorkGroupOpenState>,
    member_paths: &[String],
) -> (WorkGroupOpenState, Vec<String>) {
    let mut next = open_state.cloned().unwrap_or_else(create_empty_open_state);
    let mut opened = Vec::new();

    for member_path in unique_member_paths(member_paths) {
        if open_state_has_exact_path(&next, &member_path) {
            continue;
        }
        next.tab_groups
            .push(vec![create_directory_open_tab(&member_path)]);
        opened.push(member_path);
    }

    if !next.tab_groups.is_empty() {
        next.current_tab_group_index = next
            .current_tab_group_index
            .min(next.tab_groups.len().saturating_sub(1));
        let group_len = next
            .tab_groups
            .get(next.current_tab_group_index)
            .map(|g| g.len())
            .unwrap_or(0);
        next.current_tab_index = if group_len == 0 {
            0
        } else {
            next.current_tab_index.min(group_len - 1)
        };
    } else {
        next.current_tab_group_index = 0;
        next.current_tab_index = 0;
    }

    (next, opened)
}

pub fn add_member_to_hard_config(
    work_group: &WorkGroup,
    live_open_state: Option<&WorkGroupOpenState>,
    member_path: &str,
) -> (WorkGroup, WorkGroupOpenState, Option<WorkGroupSoftSyncNotice>) {
    let mut members = work_group.member_paths.clone();
    members.push(member_path.to_string());
    let next_members = unique_member_paths(&members);
    let next_work_group = WorkGroup {
        member_paths: next_members,
        ..work_group.clone()
    };

    let (next_open, opened) =
        ensure_member_tabs_in_open_state(live_open_state, &[member_path.to_string()]);
    let notice = if opened.is_empty() {
        None
    } else {
        Some(WorkGroupSoftSyncNotice {
            kind: "hard-members-synced-into-runtime".to_string(),
            work_group_id: work_group.id.clone(),
            opened_member_paths: opened,
        })
    };
    (next_work_group, next_open, notice)
}

/// Remove one hard member by exact path match (slash/case-insensitive). Open State tabs are kept.
pub fn remove_member_from_hard_config(
    work_group: &WorkGroup,
    member_path: &str,
) -> Result<WorkGroup, String> {
    let trimmed = member_path.trim();
    if trimmed.is_empty() {
        return Err("Member path must not be empty".to_string());
    }
    if !is_exact_member_path(trimmed, &work_group.member_paths) {
        return Err(format!("Member not found: {trimmed}"));
    }
    let next_members: Vec<String> = work_group
        .member_paths
        .iter()
        .filter(|path| !is_exact_member_path(trimmed, &[(*path).clone()]))
        .cloned()
        .collect();
    Ok(WorkGroup {
        member_paths: next_members,
        ..work_group.clone()
    })
}

pub fn rename_work_group_in_list(
    work_groups: &[WorkGroup],
    work_group_id: &str,
    name: &str,
) -> Result<Vec<WorkGroup>, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Work Group name must not be empty".to_string());
    }
    let mut next = work_groups.to_vec();
    let index = next
        .iter()
        .position(|group| group.id == work_group_id)
        .ok_or_else(|| format!("Unknown Work Group: {work_group_id}"))?;
    next[index].name = trimmed.to_string();
    Ok(next)
}

pub fn delete_work_group_in_list(
    document: &UserWorkGroups,
    work_group_id: &str,
) -> Result<UserWorkGroups, String> {
    let target = document
        .work_groups
        .iter()
        .find(|group| group.id == work_group_id)
        .ok_or_else(|| format!("Unknown Work Group: {work_group_id}"))?;
    if target.is_default {
        return Err("DEFAULT_WORK_GROUP_FORBIDDEN".to_string());
    }
    let next_groups: Vec<WorkGroup> = document
        .work_groups
        .iter()
        .filter(|group| group.id != work_group_id)
        .cloned()
        .collect();
    let next_active = if document.active_work_group_id == work_group_id {
        next_groups
            .iter()
            .find(|group| group.is_default)
            .map(|group| group.id.clone())
            .or_else(|| next_groups.first().map(|group| group.id.clone()))
            .unwrap_or_else(|| "default-work-group".to_string())
    } else {
        document.active_work_group_id.clone()
    };
    Ok(UserWorkGroups {
        work_groups: next_groups,
        active_work_group_id: next_active,
        ..document.clone()
    })
}

pub fn create_work_group_in_list(
    work_groups: &[WorkGroup],
    name: Option<String>,
) -> (Vec<WorkGroup>, WorkGroup) {
    let created = WorkGroup {
        id: next_id("work-group"),
        name: name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("Work Group {}", work_groups.len() + 1)),
        is_default: false,
        member_paths: Vec::new(),
        open_state: None,
    };
    let mut next = work_groups.to_vec();
    next.push(created.clone());
    (next, created)
}

/// Create a Work Group and silent-switch so it becomes the active selection
/// (AIA-247: user can keep editing / rename immediately after create).
pub fn create_and_activate_work_group(
    document: &UserWorkGroups,
    name: Option<String>,
    live_open_state: Option<&WorkGroupOpenState>,
) -> Result<(SilentSwitchResult, WorkGroup), String> {
    let (groups, created) = create_work_group_in_list(&document.work_groups, name);
    let mut next = document.clone();
    next.work_groups = groups;
    let switch = silent_switch_work_group(&next, &created.id, live_open_state)?;
    Ok((switch, created))
}

pub fn save_runtime_open_state_to_hard(
    work_group: &WorkGroup,
    live_open_state: Option<&WorkGroupOpenState>,
) -> WorkGroup {
    WorkGroup {
        open_state: live_open_state.cloned(),
        ..work_group.clone()
    }
}

pub fn save_members_from_open_state_to_hard(
    work_group: &WorkGroup,
    live_open_state: Option<&WorkGroupOpenState>,
) -> WorkGroup {
    let mut paths = Vec::new();
    if let Some(open_state) = live_open_state {
        for group in &open_state.tab_groups {
            for tab in group {
                if tab.tab_type == "directory" {
                    paths.push(tab.path.clone());
                }
            }
        }
    }
    WorkGroup {
        member_paths: unique_member_paths(&paths),
        ..work_group.clone()
    }
}

pub fn silent_switch_work_group(
    document: &UserWorkGroups,
    target_work_group_id: &str,
    live_open_state: Option<&WorkGroupOpenState>,
) -> Result<SilentSwitchResult, String> {
    let target = document
        .work_groups
        .iter()
        .find(|group| group.id == target_work_group_id)
        .ok_or_else(|| format!("Unknown Work Group: {target_work_group_id}"))?;

    if target.id == document.active_work_group_id {
        return Ok(SilentSwitchResult {
            document: document.clone(),
            open_state_to_restore: live_open_state.cloned(),
            require_save_prompt: false,
            relative_member_paths_to_open: Vec::new(),
            notice: None,
        });
    }

    let previous = document
        .work_groups
        .iter()
        .find(|group| group.id == document.active_work_group_id);
    let previous_members = previous
        .map(|group| group.member_paths.clone())
        .unwrap_or_default();

    let next_groups: Vec<WorkGroup> = document
        .work_groups
        .iter()
        .map(|group| {
            if group.id == document.active_work_group_id {
                save_runtime_open_state_to_hard(group, live_open_state)
            } else {
                group.clone()
            }
        })
        .collect();

    let relative: Vec<String> = target
        .member_paths
        .iter()
        .filter(|path| !is_exact_member_path(path, &previous_members))
        .cloned()
        .collect();

    let restored = target.open_state.clone();
    let (restored_with_members, opened) =
        ensure_member_tabs_in_open_state(restored.as_ref(), &relative);
    let notice = if opened.is_empty() {
        None
    } else {
        Some(WorkGroupSoftSyncNotice {
            kind: "hard-members-synced-into-runtime".to_string(),
            work_group_id: target.id.clone(),
            opened_member_paths: opened,
        })
    };

    Ok(SilentSwitchResult {
        document: UserWorkGroups {
            work_groups: next_groups,
            active_work_group_id: target.id.clone(),
        },
        open_state_to_restore: Some(restored_with_members),
        require_save_prompt: false,
        relative_member_paths_to_open: relative,
        notice,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_state(paths: &[&str]) -> WorkGroupOpenState {
        WorkGroupOpenState {
            tab_groups: paths
                .iter()
                .enumerate()
                .map(|(index, path)| {
                    vec![WorkGroupOpenStateTab {
                        id: format!("tab-{index}"),
                        name: (*path).to_string(),
                        path: (*path).to_string(),
                        tab_type: "directory".to_string(),
                    }]
                })
                .collect(),
            current_tab_group_index: 0,
            current_tab_index: 0,
        }
    }

    #[test]
    fn hard_member_add_syncs_tab_without_closing_others() {
        let group = WorkGroup {
            id: "wg-a".to_string(),
            name: "A".to_string(),
            is_default: false,
            member_paths: vec!["D:/keep".to_string()],
            open_state: None,
        };
        let live = open_state(&["D:/keep", "D:/scratch"]);
        let (next_group, next_open, notice) =
            add_member_to_hard_config(&group, Some(&live), "D:/new");
        assert_eq!(next_group.member_paths, vec!["D:/keep", "D:/new"]);
        assert!(open_state_has_exact_path(&next_open, "D:/new"));
        assert!(open_state_has_exact_path(&next_open, "D:/scratch"));
        assert_eq!(
            notice.as_ref().map(|n| n.kind.as_str()),
            Some("hard-members-synced-into-runtime")
        );
    }

    #[test]
    fn silent_switch_flushes_and_restores_without_nag() {
        let default = create_default_work_group(Some(open_state(&["D:/default"])));
        let previous = WorkGroup {
            id: "wg-prev".to_string(),
            name: "Prev".to_string(),
            is_default: false,
            member_paths: vec!["D:/shared".to_string()],
            open_state: Some(open_state(&["D:/stale"])),
        };
        let target = WorkGroup {
            id: "wg-target".to_string(),
            name: "Target".to_string(),
            is_default: false,
            member_paths: vec!["D:/target-only".to_string(), "D:/shared".to_string()],
            open_state: Some(open_state(&["D:/target-tab"])),
        };
        let document = UserWorkGroups {
            work_groups: vec![default, previous.clone(), target.clone()],
            active_work_group_id: previous.id.clone(),
        };
        let live = open_state(&["D:/live-left"]);
        let result = silent_switch_work_group(&document, &target.id, Some(&live)).unwrap();
        assert!(!result.require_save_prompt);
        assert_eq!(result.document.active_work_group_id, target.id);
        let flushed = result
            .document
            .work_groups
            .iter()
            .find(|g| g.id == previous.id)
            .unwrap();
        assert_eq!(
            flushed.open_state.as_ref().unwrap().tab_groups[0][0].path,
            "D:/live-left"
        );
        assert_eq!(
            result.open_state_to_restore.as_ref().unwrap().tab_groups[0][0].path,
            "D:/target-tab"
        );
        assert!(result
            .relative_member_paths_to_open
            .iter()
            .any(|p| p == "D:/target-only"));
    }

    #[test]
    fn create_work_group_does_not_bind_projects() {
        let (list, created) =
            create_work_group_in_list(&[create_default_work_group(None)], Some("Focus".into()));
        assert_eq!(list.len(), 2);
        assert_eq!(created.name, "Focus");
        assert!(!created.is_default);
    }

    #[test]
    fn create_and_activate_selects_new_group() {
        let previous = create_default_work_group(Some(open_state(&["D:/prev"])));
        let target_open = open_state(&["D:/keep"]);
        let document = UserWorkGroups {
            work_groups: vec![previous.clone()],
            active_work_group_id: previous.id.clone(),
        };
        let live = open_state(&["D:/live"]);
        let (switch, created) =
            create_and_activate_work_group(&document, Some("Focus".into()), Some(&live)).unwrap();
        assert_eq!(created.name, "Focus");
        assert_eq!(switch.document.active_work_group_id, created.id);
        assert!(!switch.require_save_prompt);
        let flushed = switch
            .document
            .work_groups
            .iter()
            .find(|g| g.id == previous.id)
            .unwrap();
        assert_eq!(
            flushed.open_state.as_ref().unwrap().tab_groups[0][0].path,
            "D:/live"
        );
        // New group has no saved open state / members → empty restore strip.
        let restored = switch.open_state_to_restore.expect("restore payload");
        assert!(restored.tab_groups.is_empty());
        assert!(switch.relative_member_paths_to_open.is_empty());
        let _ = target_open;
    }
}
