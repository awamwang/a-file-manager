// SPDX-License-Identifier: GPL-3.0-or-later

use super::ops::{basename_label, now_millis, next_id};
use super::types::{UserProject, UserProjects, WorkspaceLink, SCHEMA_VERSION};
use serde_json::Value;

/// Migrate raw JSON (greenfield v1 or legacy single-folder bookmark) into current schema.
pub fn migrate_value(raw: Value) -> Result<UserProjects, String> {
    if let Ok(document) = serde_json::from_value::<UserProjects>(raw.clone()) {
        return Ok(normalize_document(document));
    }

    let obj = raw
        .as_object()
        .ok_or_else(|| "projects JSON must be an object".to_string())?;

    let projects_value = obj
        .get("projects")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let projects_array = projects_value
        .as_array()
        .ok_or_else(|| "projects field must be an array".to_string())?;

    let mut projects = Vec::new();
    for entry in projects_array {
        projects.push(migrate_legacy_project(entry)?);
    }

    let active = obj
        .get("activeProjectId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    Ok(normalize_document(UserProjects {
        schema_version: SCHEMA_VERSION,
        projects,
        active_project_id: active,
    }))
}

fn migrate_legacy_project(entry: &Value) -> Result<UserProject, String> {
    if let Ok(project) = serde_json::from_value::<UserProject>(entry.clone()) {
        return Ok(project);
    }

    let obj = entry
        .as_object()
        .ok_or_else(|| "legacy project must be an object".to_string())?;

    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let id = if id.is_empty() {
        next_id("project")
    } else {
        id
    };
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();
    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let path = obj
        .get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());
    let created_at = obj
        .get("createdAt")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(now_millis);
    let updated_at = obj
        .get("updatedAt")
        .and_then(|v| v.as_u64())
        .unwrap_or(created_at);

    let mut workspace_links = Vec::new();
    if let Some(path) = path {
        workspace_links.push(WorkspaceLink {
            id: next_id("link"),
            label: basename_label(&path),
            path,
            kind: "folder".to_string(),
            added_at: created_at,
        });
    }

    Ok(UserProject {
        id,
        name,
        description,
        workspace_links,
        tasks: Vec::new(),
        context_notes: String::new(),
        created_at,
        updated_at,
    })
}

pub fn normalize_document(mut document: UserProjects) -> UserProjects {
    document.schema_version = SCHEMA_VERSION;
    if let Some(active) = document.active_project_id.clone() {
        if !document.projects.iter().any(|p| p.id == active) {
            document.active_project_id = document.projects.first().map(|p| p.id.clone());
        }
    }
    document
}

pub fn empty_document() -> UserProjects {
    UserProjects {
        schema_version: SCHEMA_VERSION,
        projects: Vec::new(),
        active_project_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migrates_legacy_single_folder_project() {
        let raw = json!({
            "projects": [{
                "id": "p1",
                "name": "Alpha",
                "path": "D:/Repos/Alpha",
                "createdAt": 100,
                "updatedAt": 200
            }],
            "activeProjectId": "p1"
        });
        let doc = migrate_value(raw).unwrap();
        assert_eq!(doc.schema_version, SCHEMA_VERSION);
        assert_eq!(doc.projects.len(), 1);
        assert_eq!(doc.projects[0].workspace_links.len(), 1);
        assert_eq!(doc.projects[0].workspace_links[0].path, "D:/Repos/Alpha");
        assert_eq!(doc.projects[0].context_notes, "");
        assert!(doc.projects[0].tasks.is_empty());
    }
}
