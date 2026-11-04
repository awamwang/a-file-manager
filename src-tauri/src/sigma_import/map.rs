// SPDX-License-Identifier: GPL-3.0-or-later

use super::discover::SigmaSourcePaths;
use super::types::{ImportMode, ImportPlan, LossyField};
use crate::projects::migrate::migrate_value;
use crate::projects::types::UserProjects;
use crate::work_groups::sync::default_document;
use crate::work_groups::types::{
    UserWorkGroups, WorkGroup, WorkGroupOpenState, WorkGroupOpenStateTab, DEFAULT_WORK_GROUP_ID,
};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MappedImport {
    pub work_groups: UserWorkGroups,
    pub projects: UserProjects,
    pub navigator_open_state: Option<WorkGroupOpenState>,
    pub report: MappingReport,
}

#[derive(Debug, Clone)]
pub struct MappingReport {
    pub lossy_fields: Vec<LossyField>,
    pub notes: Vec<String>,
    pub navigator_tab_group_count: usize,
}

pub fn map_sigma_bundle(source: &SigmaSourcePaths) -> Result<MappedImport, String> {
    let wg_raw = read_json(&source.work_groups)?;
    let projects_raw = if source.projects.is_file() {
        Some(read_json(&source.projects)?)
    } else {
        None
    };
    let workspaces_raw = if source.workspaces.is_file() {
        Some(read_json(&source.workspaces)?)
    } else {
        None
    };
    map_sigma_values(wg_raw, projects_raw, workspaces_raw)
}

pub fn map_sigma_values(
    work_groups_raw: Value,
    projects_raw: Option<Value>,
    workspaces_raw: Option<Value>,
) -> Result<MappedImport, String> {
    let mut lossy = Vec::new();
    let mut notes = Vec::new();

    let (work_groups, mut wg_lossy) = map_work_groups(work_groups_raw)?;
    lossy.append(&mut wg_lossy);

    let projects = match projects_raw {
        Some(raw) => {
            let (doc, mut project_lossy) = map_projects(raw)?;
            lossy.append(&mut project_lossy);
            doc
        }
        None => {
            notes.push("user-projects.json missing; projects left empty".into());
            crate::projects::migrate::empty_document()
        }
    };

    let (navigator_open_state, tab_group_count, mut nav_lossy) =
        map_navigator_session(workspaces_raw.as_ref())?;
    lossy.append(&mut nav_lossy);
    if navigator_open_state.is_none() {
        notes.push(
            "No Navigator session in user-workspaces.json; active Work Group openState used as-is"
                .into(),
        );
    } else {
        notes.push(
            "Live Navigator session (user-workspaces) will overlay active Work Group openState on apply"
                .into(),
        );
    }

    // Deduplicate lossy reasons for stable reports
    lossy.sort_by(|a, b| (&a.surface, &a.field).cmp(&(&b.surface, &b.field)));
    lossy.dedup();

    Ok(MappedImport {
        work_groups,
        projects,
        navigator_open_state,
        report: MappingReport {
            lossy_fields: lossy,
            notes,
            navigator_tab_group_count: tab_group_count,
        },
    })
}

pub fn build_plan(mapped: &MappedImport, mode: ImportMode) -> ImportPlan {
    ImportPlan {
        mode,
        work_group_count: mapped.work_groups.work_groups.len(),
        project_count: mapped.projects.projects.len(),
        navigator_tab_group_count: mapped.report.navigator_tab_group_count,
        lossy_fields: mapped.report.lossy_fields.clone(),
        notes: mapped.report.notes.clone(),
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse {}: {err}", path.display()))
}

fn map_work_groups(raw: Value) -> Result<(UserWorkGroups, Vec<LossyField>), String> {
    let mut lossy = Vec::new();
    let obj = raw
        .as_object()
        .ok_or_else(|| "user-work-groups.json must be an object".to_string())?;

    let groups_value = obj
        .get("workGroups")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let groups_array = groups_value
        .as_array()
        .ok_or_else(|| "workGroups must be an array".to_string())?;

    let mut work_groups = Vec::new();
    for entry in groups_array {
        let (group, mut entry_lossy) = map_one_work_group(entry)?;
        lossy.append(&mut entry_lossy);
        work_groups.push(group);
    }

    if work_groups.is_empty() {
        return Ok((default_document(), lossy));
    }

    let active = obj
        .get("activeWorkGroupId")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_WORK_GROUP_ID)
        .to_string();
    let active = if work_groups.iter().any(|g| g.id == active) {
        active
    } else {
        work_groups[0].id.clone()
    };

    // Ensure a default flag exists
    if !work_groups.iter().any(|g| g.is_default) {
        if let Some(default) = work_groups
            .iter_mut()
            .find(|g| g.id == DEFAULT_WORK_GROUP_ID)
        {
            default.is_default = true;
        } else {
            work_groups[0].is_default = true;
        }
    }

    Ok((
        UserWorkGroups {
            work_groups,
            active_work_group_id: active,
        },
        lossy,
    ))
}

fn map_one_work_group(entry: &Value) -> Result<(WorkGroup, Vec<LossyField>), String> {
    let mut lossy = Vec::new();
    let obj = entry
        .as_object()
        .ok_or_else(|| "work group entry must be an object".to_string())?;

    if obj.get("navigatorDefaults").map(|v| !v.is_null()).unwrap_or(false) {
        lossy.push(LossyField {
            surface: "workGroups".into(),
            field: "navigatorDefaults".into(),
            reason: "AFM WorkGroup schema has no Group Navigator Defaults yet; dropped on import"
                .into(),
        });
    }

    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_WORK_GROUP_ID)
        .to_string();
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();
    let is_default = obj
        .get("isDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(id == DEFAULT_WORK_GROUP_ID);
    let member_paths = obj
        .get("memberPaths")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let open_state = match obj.get("openState") {
        None | Some(Value::Null) => None,
        Some(state) => {
            let (mapped, mut state_lossy) = map_open_state(state)?;
            lossy.append(&mut state_lossy);
            Some(mapped)
        }
    };

    Ok((
        WorkGroup {
            id,
            name,
            is_default,
            member_paths,
            open_state,
        },
        lossy,
    ))
}

fn map_open_state(raw: &Value) -> Result<(WorkGroupOpenState, Vec<LossyField>), String> {
    let mut lossy = Vec::new();
    let obj = raw
        .as_object()
        .ok_or_else(|| "openState must be an object".to_string())?;

    let groups_value = obj
        .get("tabGroups")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let groups_array = groups_value
        .as_array()
        .ok_or_else(|| "tabGroups must be an array".to_string())?;

    let mut tab_groups = Vec::new();
    for group in groups_array {
        let tabs_array = group
            .as_array()
            .ok_or_else(|| "each tab group must be an array".to_string())?;
        let mut tabs = Vec::new();
        for tab in tabs_array {
            let (mapped, mut tab_lossy) = map_open_tab(tab)?;
            lossy.append(&mut tab_lossy);
            tabs.push(mapped);
        }
        tab_groups.push(tabs);
    }

    let current_tab_group_index = obj
        .get("currentTabGroupIndex")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let current_tab_index = obj
        .get("currentTabIndex")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    Ok((
        WorkGroupOpenState {
            tab_groups,
            current_tab_group_index,
            current_tab_index,
        },
        lossy,
    ))
}

fn map_open_tab(raw: &Value) -> Result<(WorkGroupOpenStateTab, Vec<LossyField>), String> {
    let mut lossy = Vec::new();
    let obj = raw
        .as_object()
        .ok_or_else(|| "openState tab must be an object".to_string())?;

    if obj.get("paneWidth").is_some() {
        lossy.push(LossyField {
            surface: "workGroups.openState.tab".into(),
            field: "paneWidth".into(),
            reason: "AFM Open State tabs do not persist split paneWidth yet".into(),
        });
    }
    if obj
        .get("filterQuery")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false)
    {
        lossy.push(LossyField {
            surface: "workGroups.openState.tab".into(),
            field: "filterQuery".into(),
            reason: "AFM Open State tabs do not persist filterQuery yet".into(),
        });
    }

    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let path = obj
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let tab_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("directory")
        .to_string();

    if id.is_empty() || path.is_empty() {
        return Err("openState tab requires id and path".into());
    }

    Ok((
        WorkGroupOpenStateTab {
            id,
            name,
            path,
            tab_type,
        },
        lossy,
    ))
}

fn map_projects(raw: Value) -> Result<(UserProjects, Vec<LossyField>), String> {
    let lossy = canonical_project_lossy_fields(&raw);
    // migrate_value drops Sigma-only fields via serde / normalize
    let mut document = migrate_value(raw)?;
    document.schema_version = crate::projects::types::SCHEMA_VERSION;
    Ok((document, lossy))
}

fn canonical_project_lossy_fields(raw: &Value) -> Vec<LossyField> {
    let mut lossy = Vec::new();
    let Some(projects) = raw.get("projects").and_then(|v| v.as_array()) else {
        return lossy;
    };
    let sigma_only = [
        ("path", "Deprecated single-folder path; AFM uses workspaceLinks only"),
        ("tagIds", "AFM Project schema has no tags yet"),
        ("ideCommand", "AFM Project schema has no IDE launch command yet"),
        (
            "terminalProfile",
            "AFM Project schema has no terminal profile yet",
        ),
        ("addedAt", "AFM keeps createdAt/updatedAt only"),
        ("lastOpenedAt", "AFM Project schema has no open stats yet"),
        ("openCount", "AFM Project schema has no open stats yet"),
        (
            "tasksViewMode",
            "AFM UI toggles list/board in-session; not persisted yet",
        ),
    ];
    for project in projects {
        let Some(obj) = project.as_object() else {
            continue;
        };
        for (field, reason) in sigma_only {
            let present = match obj.get(field) {
                Some(Value::Null) => false,
                Some(Value::Array(a)) => !a.is_empty(),
                Some(Value::String(s)) => !s.is_empty(),
                Some(_) => true,
                None => false,
            };
            if present {
                lossy.push(LossyField {
                    surface: "projects".into(),
                    field: field.into(),
                    reason: reason.into(),
                });
            }
        }
    }
    lossy.sort_by(|a, b| a.field.cmp(&b.field));
    lossy.dedup();
    lossy
}

fn map_navigator_session(
    workspaces_raw: Option<&Value>,
) -> Result<(Option<WorkGroupOpenState>, usize, Vec<LossyField>), String> {
    let mut lossy = Vec::new();
    let Some(raw) = workspaces_raw else {
        return Ok((None, 0, lossy));
    };
    let obj = raw
        .as_object()
        .ok_or_else(|| "user-workspaces.json must be an object".to_string())?;

    let workspaces = obj
        .get("workspaces")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if workspaces.is_empty() {
        return Ok((None, 0, lossy));
    }

    if workspaces.len() > 1 {
        lossy.push(LossyField {
            surface: "navigator.session".into(),
            field: "workspaces[1..]".into(),
            reason: "AFM has a single Navigator session plane; only primary/current workspace imported"
                .into(),
        });
    }

    let primary = workspaces
        .iter()
        .find(|w| {
            w.get("isCurrent")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || w.get("isPrimary")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        })
        .or_else(|| workspaces.first())
        .ok_or_else(|| "no workspace entry".to_string())?;

    if primary
        .get("actions")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        lossy.push(LossyField {
            surface: "navigator.session".into(),
            field: "actions".into(),
            reason: "Workspace actions are Sigma-only; dropped".into(),
        });
    }

    let (open_state, mut state_lossy) = map_open_state(primary)?;
    lossy.append(&mut state_lossy);
    let count = open_state.tab_groups.len();
    Ok((Some(open_state), count, lossy))
}

/// Merge mapped Sigma into existing AFM documents without deleting AFM-only ids.
pub fn merge_documents(
    existing_wg: &UserWorkGroups,
    existing_projects: &UserProjects,
    mapped: &MappedImport,
) -> (UserWorkGroups, UserProjects) {
    let mut work_groups = existing_wg.work_groups.clone();
    let existing_ids: std::collections::HashSet<_> =
        work_groups.iter().map(|g| g.id.clone()).collect();

    for incoming in &mapped.work_groups.work_groups {
        if existing_ids.contains(&incoming.id) {
            // Fill empty openState only
            if let Some(existing) = work_groups.iter_mut().find(|g| g.id == incoming.id) {
                if existing.open_state.as_ref().map(|s| s.tab_groups.is_empty()).unwrap_or(true)
                {
                    if let Some(open) = &incoming.open_state {
                        existing.open_state = Some(open.clone());
                    }
                }
                for path in &incoming.member_paths {
                    if !existing.member_paths.iter().any(|p| p == path) {
                        existing.member_paths.push(path.clone());
                    }
                }
            }
            continue;
        }
        work_groups.push(incoming.clone());
    }

    let mut active = existing_wg.active_work_group_id.clone();
    if !work_groups.iter().any(|g| g.id == active) {
        active = mapped.work_groups.active_work_group_id.clone();
    }

    // Overlay navigator session onto active WG when its openState is empty
    let mut document = UserWorkGroups {
        work_groups,
        active_work_group_id: active.clone(),
    };
    if let Some(session) = &mapped.navigator_open_state {
        if let Some(active_wg) = document
            .work_groups
            .iter_mut()
            .find(|g| g.id == active)
        {
            if active_wg
                .open_state
                .as_ref()
                .map(|s| s.tab_groups.is_empty())
                .unwrap_or(true)
            {
                active_wg.open_state = Some(session.clone());
            }
        }
    }

    let mut projects = existing_projects.projects.clone();
    let project_ids: std::collections::HashSet<_> =
        projects.iter().map(|p| p.id.clone()).collect();
    for incoming in &mapped.projects.projects {
        if !project_ids.contains(&incoming.id) {
            projects.push(incoming.clone());
        }
    }
    let mut active_project = existing_projects.active_project_id.clone();
    if active_project.is_none() {
        active_project = mapped.projects.active_project_id.clone();
    }
    if let Some(id) = &active_project {
        if !projects.iter().any(|p| &p.id == id) {
            active_project = projects.first().map(|p| p.id.clone());
        }
    }

    (
        document,
        UserProjects {
            schema_version: crate::projects::types::SCHEMA_VERSION,
            projects,
            active_project_id: active_project,
        },
    )
}

/// Replace AFM documents with mapped Sigma; overlay live Navigator session on active WG.
pub fn replace_documents(mapped: &MappedImport) -> (UserWorkGroups, UserProjects) {
    let mut work_groups = mapped.work_groups.clone();
    if let Some(session) = &mapped.navigator_open_state {
        let active = work_groups.active_work_group_id.clone();
        if let Some(active_wg) = work_groups
            .work_groups
            .iter_mut()
            .find(|g| g.id == active)
        {
            active_wg.open_state = Some(session.clone());
        }
    }
    let mut projects = mapped.projects.clone();
    projects.schema_version = crate::projects::types::SCHEMA_VERSION;
    (work_groups, projects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_work_groups_and_drops_navigator_defaults() {
        let raw = json!({
            "activeWorkGroupId": "default-work-group",
            "workGroups": [{
                "id": "default-work-group",
                "name": "Default",
                "isDefault": true,
                "memberPaths": ["D:/Repos"],
                "navigatorDefaults": { "layout": "list", "showHiddenFiles": false },
                "openState": {
                    "currentTabGroupIndex": 0,
                    "currentTabIndex": 0,
                    "tabGroups": [[
                        {
                            "id": "t1",
                            "name": "Repos",
                            "path": "D:/Repos",
                            "type": "directory",
                            "paneWidth": 100,
                            "filterQuery": ""
                        }
                    ]]
                }
            }]
        });
        let mapped = map_sigma_values(raw, None, None).unwrap();
        assert_eq!(mapped.work_groups.work_groups.len(), 1);
        assert_eq!(
            mapped.work_groups.work_groups[0].member_paths,
            vec!["D:/Repos".to_string()]
        );
        assert!(mapped
            .report
            .lossy_fields
            .iter()
            .any(|f| f.field == "navigatorDefaults"));
        assert!(mapped
            .report
            .lossy_fields
            .iter()
            .any(|f| f.field == "paneWidth"));
        let tabs = mapped.work_groups.work_groups[0]
            .open_state
            .as_ref()
            .unwrap();
        assert_eq!(tabs.tab_groups[0][0].path, "D:/Repos");
    }

    #[test]
    fn maps_sigma_v2_project_and_reports_lossy_stats() {
        let wg = json!({
            "workGroups": [{
                "id": "default-work-group",
                "name": "Default",
                "isDefault": true,
                "memberPaths": [],
                "openState": null
            }],
            "activeWorkGroupId": "default-work-group"
        });
        let projects = json!({
            "schemaVersion": 2,
            "activeProjectId": "project-1",
            "projects": [{
                "id": "project-1",
                "name": "OpenMontage",
                "description": "",
                "contextNotes": "notes",
                "createdAt": 100,
                "updatedAt": 200,
                "path": "G:/Projects/Demo",
                "tagIds": [],
                "ideCommand": null,
                "terminalProfile": null,
                "addedAt": 100,
                "lastOpenedAt": 300,
                "openCount": 2,
                "tasksViewMode": "list",
                "tasks": [],
                "workspaceLinks": [{
                    "id": "link-1",
                    "path": "G:/Projects/Demo",
                    "label": "Demo",
                    "kind": "folder",
                    "addedAt": 100
                }]
            }]
        });
        let mapped = map_sigma_values(wg, Some(projects), None).unwrap();
        assert_eq!(mapped.projects.projects.len(), 1);
        assert_eq!(
            mapped.projects.schema_version,
            crate::projects::types::SCHEMA_VERSION
        );
        assert!(mapped.projects.projects[0].work_group_id.is_none());
        assert_eq!(
            mapped.projects.projects[0].workspace_links[0].path,
            "G:/Projects/Demo"
        );
        assert_eq!(mapped.projects.projects[0].context_notes, "notes");
        assert!(mapped
            .report
            .lossy_fields
            .iter()
            .any(|f| f.field == "openCount"));
        assert!(mapped
            .report
            .lossy_fields
            .iter()
            .any(|f| f.field == "tasksViewMode"));
    }

    #[test]
    fn overlays_navigator_session_on_replace() {
        let wg = json!({
            "workGroups": [{
                "id": "default-work-group",
                "name": "Default",
                "isDefault": true,
                "memberPaths": [],
                "openState": {
                    "currentTabGroupIndex": 0,
                    "currentTabIndex": 0,
                    "tabGroups": [[{"id":"old","name":"Old","path":"D:/Old","type":"directory"}]]
                }
            }],
            "activeWorkGroupId": "default-work-group"
        });
        let workspaces = json!({
            "__schemaVersion": 1,
            "workspaces": [{
                "id": 0,
                "isCurrent": true,
                "isPrimary": true,
                "name": "primary",
                "actions": [],
                "currentTabGroupIndex": 0,
                "currentTabIndex": 0,
                "tabGroups": [[{"id":"live","name":"Live","path":"D:/Live","type":"directory","paneWidth":100,"filterQuery":""}]]
            }]
        });
        let mapped = map_sigma_values(wg, None, Some(workspaces)).unwrap();
        let (wg_doc, _) = replace_documents(&mapped);
        let open = wg_doc.work_groups[0].open_state.as_ref().unwrap();
        assert_eq!(open.tab_groups[0][0].path, "D:/Live");
        assert_eq!(mapped.report.navigator_tab_group_count, 1);
    }

    #[test]
    fn merge_keeps_existing_project_and_adds_missing_wg() {
        let existing_wg = default_document();
        let existing_projects = crate::projects::migrate::empty_document();
        let wg = json!({
            "workGroups": [
                {
                    "id": "default-work-group",
                    "name": "Default",
                    "isDefault": true,
                    "memberPaths": ["D:/A"],
                    "openState": null
                },
                {
                    "id": "work-group-extra",
                    "name": "Extra",
                    "isDefault": false,
                    "memberPaths": ["D:/B"],
                    "openState": null
                }
            ],
            "activeWorkGroupId": "work-group-extra"
        });
        let projects = json!({
            "schemaVersion": 2,
            "activeProjectId": "p1",
            "projects": [{
                "id": "p1",
                "name": "P",
                "description": "",
                "contextNotes": "",
                "createdAt": 1,
                "updatedAt": 1,
                "tasks": [],
                "workspaceLinks": []
            }]
        });
        let mapped = map_sigma_values(wg, Some(projects), None).unwrap();
        let (wg_doc, proj_doc) = merge_documents(&existing_wg, &existing_projects, &mapped);
        assert!(wg_doc.work_groups.iter().any(|g| g.id == "work-group-extra"));
        assert_eq!(
            wg_doc
                .work_groups
                .iter()
                .find(|g| g.id == DEFAULT_WORK_GROUP_ID)
                .unwrap()
                .member_paths,
            vec!["D:/A".to_string()]
        );
        assert_eq!(proj_doc.projects.len(), 1);
    }
}
