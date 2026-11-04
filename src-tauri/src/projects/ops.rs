// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::{
    OpenAllLinksResult, ProjectTask, TaskPatch, UserProject, UserProjects, WorkspaceLink,
    TASK_STATUSES,
};
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

fn normalize_path(path: &str) -> String {
    path.trim().replace('/', "\\")
}

fn paths_equal(a: &str, b: &str) -> bool {
    normalize_path(a).eq_ignore_ascii_case(&normalize_path(b))
}

fn validate_status(status_id: &str) -> Result<(), String> {
    if TASK_STATUSES.contains(&status_id) {
        Ok(())
    } else {
        Err(format!("Unknown task status: {status_id}"))
    }
}

fn find_project_mut<'a>(
    document: &'a mut UserProjects,
    project_id: &str,
) -> Result<&'a mut UserProject, String> {
    document
        .projects
        .iter_mut()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("Unknown Project: {project_id}"))
}

pub fn create_project(
    document: &UserProjects,
    name: Option<String>,
    description: Option<String>,
) -> (UserProjects, UserProject) {
    let stamp = now_millis();
    let created = UserProject {
        id: next_id("project"),
        name: name
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| format!("Project {}", document.projects.len() + 1)),
        description: description.unwrap_or_default(),
        workspace_links: Vec::new(),
        tasks: Vec::new(),
        context_notes: String::new(),
        created_at: stamp,
        updated_at: stamp,
    };
    let mut next = document.clone();
    next.projects.push(created.clone());
    if next.active_project_id.is_none() {
        next.active_project_id = Some(created.id.clone());
    }
    (next, created)
}

pub fn rename_project(
    document: &UserProjects,
    project_id: &str,
    name: String,
    description: Option<String>,
) -> Result<UserProjects, String> {
    let mut next = document.clone();
    let project = find_project_mut(&mut next, project_id)?;
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    project.name = trimmed.to_string();
    if let Some(description) = description {
        project.description = description;
    }
    project.updated_at = now_millis();
    Ok(next)
}

pub fn delete_project(document: &UserProjects, project_id: &str) -> Result<UserProjects, String> {
    if !document.projects.iter().any(|p| p.id == project_id) {
        return Err(format!("Unknown Project: {project_id}"));
    }
    let mut next = document.clone();
    next.projects.retain(|p| p.id != project_id);
    if next.active_project_id.as_deref() == Some(project_id) {
        next.active_project_id = next.projects.first().map(|p| p.id.clone());
    }
    Ok(next)
}

pub fn set_active_project(
    document: &UserProjects,
    project_id: Option<String>,
) -> Result<UserProjects, String> {
    let mut next = document.clone();
    if let Some(ref id) = project_id {
        if !next.projects.iter().any(|p| p.id == *id) {
            return Err(format!("Unknown Project: {id}"));
        }
    }
    next.active_project_id = project_id;
    Ok(next)
}

pub fn add_workspace_link(
    document: &UserProjects,
    project_id: &str,
    path: &str,
    label: Option<String>,
) -> Result<UserProjects, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Workspace link path cannot be empty".to_string());
    }
    let mut next = document.clone();
    let project = find_project_mut(&mut next, project_id)?;
    if project
        .workspace_links
        .iter()
        .any(|link| paths_equal(&link.path, trimmed))
    {
        return Ok(next);
    }
    let stamp = now_millis();
    project.workspace_links.push(WorkspaceLink {
        id: next_id("link"),
        path: trimmed.to_string(),
        label: label
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| basename_label(trimmed)),
        kind: "folder".to_string(),
        added_at: stamp,
    });
    project.updated_at = stamp;
    Ok(next)
}

pub fn remove_workspace_link(
    document: &UserProjects,
    project_id: &str,
    link_id: &str,
) -> Result<UserProjects, String> {
    let mut next = document.clone();
    let project = find_project_mut(&mut next, project_id)?;
    let before = project.workspace_links.len();
    project.workspace_links.retain(|link| link.id != link_id);
    if project.workspace_links.len() == before {
        return Err(format!("Unknown workspace link: {link_id}"));
    }
    for task in &mut project.tasks {
        if task.workspace_link_id.as_deref() == Some(link_id) {
            task.workspace_link_id = None;
            task.updated_at = now_millis();
        }
    }
    project.updated_at = now_millis();
    Ok(next)
}

pub fn set_context_notes(
    document: &UserProjects,
    project_id: &str,
    context_notes: String,
) -> Result<UserProjects, String> {
    let mut next = document.clone();
    let project = find_project_mut(&mut next, project_id)?;
    project.context_notes = context_notes;
    project.updated_at = now_millis();
    Ok(next)
}

pub fn add_task(
    document: &UserProjects,
    project_id: &str,
    title: String,
    status_id: Option<String>,
) -> Result<(UserProjects, ProjectTask), String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err("Task title cannot be empty".to_string());
    }
    let status = status_id.unwrap_or_else(|| "backlog".to_string());
    validate_status(&status)?;

    let mut next = document.clone();
    let project = find_project_mut(&mut next, project_id)?;
    let stamp = now_millis();
    let max_sort = project
        .tasks
        .iter()
        .filter(|t| t.status_id == status)
        .map(|t| t.sort_key)
        .max()
        .unwrap_or(-1);
    let task = ProjectTask {
        id: next_id("task"),
        title: trimmed.to_string(),
        body: String::new(),
        status_id: status,
        sort_key: max_sort + 1,
        workspace_link_id: None,
        outcome_url: None,
        created_at: stamp,
        updated_at: stamp,
        completed_at: None,
    };
    project.tasks.push(task.clone());
    project.updated_at = stamp;
    Ok((next, task))
}

pub fn update_task(
    document: &UserProjects,
    project_id: &str,
    task_id: &str,
    patch: TaskPatch,
) -> Result<UserProjects, String> {
    let mut next = document.clone();
    if let Some(Some(ref link_id)) = patch.workspace_link_id {
        let project = next
            .projects
            .iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| format!("Unknown Project: {project_id}"))?;
        if !project.workspace_links.iter().any(|link| link.id == *link_id) {
            return Err(format!("Unknown workspace link: {link_id}"));
        }
    }

    let project = find_project_mut(&mut next, project_id)?;
    let task = project
        .tasks
        .iter_mut()
        .find(|t| t.id == task_id)
        .ok_or_else(|| format!("Unknown task: {task_id}"))?;

    if let Some(title) = patch.title {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err("Task title cannot be empty".to_string());
        }
        task.title = trimmed.to_string();
    }
    if let Some(body) = patch.body {
        task.body = body;
    }
    if let Some(status_id) = patch.status_id {
        validate_status(&status_id)?;
        let becoming_done = status_id == "done";
        let was_done = task.status_id == "done";
        task.status_id = status_id;
        if becoming_done && !was_done {
            task.completed_at = Some(now_millis());
        } else if !becoming_done {
            task.completed_at = None;
        }
    }
    if let Some(sort_key) = patch.sort_key {
        task.sort_key = sort_key;
    }
    if let Some(workspace_link_id) = patch.workspace_link_id {
        task.workspace_link_id = workspace_link_id;
    }
    if let Some(outcome_url) = patch.outcome_url {
        task.outcome_url = outcome_url;
    }
    task.updated_at = now_millis();
    project.updated_at = task.updated_at;
    Ok(next)
}

pub fn move_task(
    document: &UserProjects,
    project_id: &str,
    task_id: &str,
    status_id: String,
    sort_key: i64,
) -> Result<UserProjects, String> {
    update_task(
        document,
        project_id,
        task_id,
        TaskPatch {
            title: None,
            body: None,
            status_id: Some(status_id),
            sort_key: Some(sort_key),
            workspace_link_id: None,
            outcome_url: None,
        },
    )
}

pub fn delete_task(
    document: &UserProjects,
    project_id: &str,
    task_id: &str,
) -> Result<UserProjects, String> {
    let mut next = document.clone();
    let project = find_project_mut(&mut next, project_id)?;
    let before = project.tasks.len();
    project.tasks.retain(|t| t.id != task_id);
    if project.tasks.len() == before {
        return Err(format!("Unknown task: {task_id}"));
    }
    project.updated_at = now_millis();
    Ok(next)
}

/// Open all workspace links as Navigator tab paths — never mutates Work Groups.
pub fn open_all_links(
    document: &UserProjects,
    project_id: &str,
) -> Result<OpenAllLinksResult, String> {
    let project = document
        .projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("Unknown Project: {project_id}"))?;
    Ok(OpenAllLinksResult {
        project_id: project.id.clone(),
        paths: project
            .workspace_links
            .iter()
            .map(|link| link.path.clone())
            .collect(),
        work_group_mutated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::migrate::empty_document;

    #[test]
    fn open_all_returns_paths_without_work_group_mutation() {
        let (mut doc, project) = create_project(&empty_document(), Some("P".into()), None);
        doc = add_workspace_link(&doc, &project.id, r"D:\A", None).unwrap();
        doc = add_workspace_link(&doc, &project.id, r"D:\B", None).unwrap();
        let result = open_all_links(&doc, &project.id).unwrap();
        assert_eq!(result.paths, vec![r"D:\A".to_string(), r"D:\B".to_string()]);
        assert!(!result.work_group_mutated);
    }

    #[test]
    fn board_move_updates_status_and_sort() {
        let (mut doc, project) = create_project(&empty_document(), Some("P".into()), None);
        let (doc2, task) = add_task(&doc, &project.id, "Ship".into(), None).unwrap();
        doc = doc2;
        doc = move_task(&doc, &project.id, &task.id, "doing".into(), 3).unwrap();
        let updated = doc.projects[0]
            .tasks
            .iter()
            .find(|t| t.id == task.id)
            .unwrap();
        assert_eq!(updated.status_id, "doing");
        assert_eq!(updated.sort_key, 3);
    }

    #[test]
    fn create_project_has_independent_links_not_work_groups() {
        let (doc, project) = create_project(&empty_document(), Some("Domain".into()), None);
        assert!(project.workspace_links.is_empty());
        assert!(project.context_notes.is_empty());
        assert_eq!(doc.projects.len(), 1);
    }
}
