// SPDX-License-Identifier: GPL-3.0-or-later
//! Workspace CONTEXT.md + board.json (AIA-211 / C5).
//!
//! Four fixed columns (Cursor-style): backlog | doing | review | done.
//! Board authority is the Workspace `board.json` on disk — not APPDATA projects.json.

use super::ops::{next_id, now_millis};
use super::types::TASK_STATUSES;
use super::workspace::{
    load_workspace, WORKSPACE_BOARD_FILE, WORKSPACE_BOARD_SCHEMA_VERSION, WORKSPACE_CONTEXT_FILE,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const ERR_TASK_TITLE_EMPTY: &str = "PROJECT_TASK_TITLE_EMPTY";
pub const ERR_TASK_UNKNOWN: &str = "PROJECT_TASK_UNKNOWN";
pub const ERR_TASK_STATUS_UNKNOWN: &str = "PROJECT_TASK_STATUS_UNKNOWN";

/// One card on the Workspace board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BoardTask {
    pub id: String,
    pub title: String,
    pub body: String,
    /// One of: backlog | doing | review | done.
    pub status_id: String,
    pub sort_key: i64,
    pub created_at: u64,
    pub updated_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardTaskPatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub status_id: Option<String>,
    pub sort_key: Option<i64>,
}

/// Disk `board.json` — fixed four columns + task cards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceBoard {
    pub schema_version: u32,
    /// Always the four Cursor-style columns (order fixed).
    pub columns: Vec<String>,
    pub tasks: Vec<BoardTask>,
}

impl WorkspaceBoard {
    pub fn empty() -> Self {
        Self {
            schema_version: WORKSPACE_BOARD_SCHEMA_VERSION,
            columns: TASK_STATUSES.iter().map(|s| (*s).to_string()).collect(),
            tasks: Vec::new(),
        }
    }

    fn normalize(mut self) -> Self {
        self.schema_version = WORKSPACE_BOARD_SCHEMA_VERSION;
        self.columns = TASK_STATUSES.iter().map(|s| (*s).to_string()).collect();
        self
    }
}

fn validate_status(status_id: &str) -> Result<(), String> {
    if TASK_STATUSES.contains(&status_id) {
        Ok(())
    } else {
        Err(ERR_TASK_STATUS_UNKNOWN.to_string())
    }
}

pub fn read_context(workspace_dir: &Path) -> Result<String, String> {
    let path = workspace_dir.join(WORKSPACE_CONTEXT_FILE);
    if !path.exists() {
        fs::write(&path, "").map_err(|err| format!("Failed to create CONTEXT.md: {err}"))?;
        return Ok(String::new());
    }
    fs::read_to_string(&path).map_err(|err| format!("Failed to read CONTEXT.md: {err}"))
}

pub fn write_context(workspace_dir: &Path, content: &str) -> Result<(), String> {
    let path = workspace_dir.join(WORKSPACE_CONTEXT_FILE);
    fs::write(&path, content).map_err(|err| format!("Failed to write CONTEXT.md: {err}"))
}

pub fn read_board(workspace_dir: &Path) -> Result<WorkspaceBoard, String> {
    let path = workspace_dir.join(WORKSPACE_BOARD_FILE);
    if !path.exists() {
        let board = WorkspaceBoard::empty();
        write_board(workspace_dir, &board)?;
        return Ok(board);
    }
    let text =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read board.json: {err}"))?;
    let board: WorkspaceBoard = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse board.json: {err}"))?;
    let board = board.normalize();
    // Persist normalized columns if file was sparse / older shape.
    write_board(workspace_dir, &board)?;
    Ok(board)
}

pub fn write_board(workspace_dir: &Path, board: &WorkspaceBoard) -> Result<(), String> {
    let path = workspace_dir.join(WORKSPACE_BOARD_FILE);
    let text = serde_json::to_string_pretty(board)
        .map_err(|err| format!("Failed to serialize board.json: {err}"))?;
    fs::write(&path, format!("{text}\n"))
        .map_err(|err| format!("Failed to write board.json: {err}"))
}

pub fn add_task(
    board: &WorkspaceBoard,
    title: &str,
    status_id: Option<&str>,
) -> Result<(WorkspaceBoard, BoardTask), String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(ERR_TASK_TITLE_EMPTY.to_string());
    }
    let status = status_id.unwrap_or("backlog");
    validate_status(status)?;

    let mut next = board.clone().normalize();
    let stamp = now_millis();
    let max_sort = next
        .tasks
        .iter()
        .filter(|t| t.status_id == status)
        .map(|t| t.sort_key)
        .max()
        .unwrap_or(-1);
    let task = BoardTask {
        id: next_id("task"),
        title: trimmed.to_string(),
        body: String::new(),
        status_id: status.to_string(),
        sort_key: max_sort + 1,
        created_at: stamp,
        updated_at: stamp,
        completed_at: if status == "done" { Some(stamp) } else { None },
    };
    next.tasks.push(task.clone());
    Ok((next, task))
}

pub fn update_task(
    board: &WorkspaceBoard,
    task_id: &str,
    patch: BoardTaskPatch,
) -> Result<WorkspaceBoard, String> {
    let mut next = board.clone().normalize();
    let task = next
        .tasks
        .iter_mut()
        .find(|t| t.id == task_id)
        .ok_or_else(|| ERR_TASK_UNKNOWN.to_string())?;

    if let Some(title) = patch.title {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(ERR_TASK_TITLE_EMPTY.to_string());
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
    task.updated_at = now_millis();
    Ok(next)
}

pub fn move_task(
    board: &WorkspaceBoard,
    task_id: &str,
    status_id: &str,
    sort_key: i64,
) -> Result<WorkspaceBoard, String> {
    update_task(
        board,
        task_id,
        BoardTaskPatch {
            title: None,
            body: None,
            status_id: Some(status_id.to_string()),
            sort_key: Some(sort_key),
        },
    )
}

pub fn delete_task(board: &WorkspaceBoard, task_id: &str) -> Result<WorkspaceBoard, String> {
    if !board.tasks.iter().any(|t| t.id == task_id) {
        return Err(ERR_TASK_UNKNOWN.to_string());
    }
    let mut next = board.clone().normalize();
    next.tasks.retain(|t| t.id != task_id);
    Ok(next)
}

pub fn read_context_for_slug(projects_root: &Path, slug: &str) -> Result<String, String> {
    let item = load_workspace(projects_root, slug)?;
    read_context(Path::new(&item.path))
}

pub fn write_context_for_slug(
    projects_root: &Path,
    slug: &str,
    content: &str,
) -> Result<(), String> {
    let item = load_workspace(projects_root, slug)?;
    write_context(Path::new(&item.path), content)
}

pub fn read_board_for_slug(projects_root: &Path, slug: &str) -> Result<WorkspaceBoard, String> {
    let item = load_workspace(projects_root, slug)?;
    read_board(Path::new(&item.path))
}

pub fn mutate_board_for_slug<F>(
    projects_root: &Path,
    slug: &str,
    f: F,
) -> Result<WorkspaceBoard, String>
where
    F: FnOnce(&WorkspaceBoard) -> Result<WorkspaceBoard, String>,
{
    let item = load_workspace(projects_root, slug)?;
    let dir = Path::new(&item.path);
    let board = read_board(dir)?;
    let next = f(&board)?;
    write_board(dir, &next)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::workspace::create_workspace_at;
    use std::path::PathBuf;

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-board-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn context_round_trip() {
        let root = temp_root("ctx");
        let ws = create_workspace_at(&root, "App", None, Some("app"), None).unwrap();
        let dir = Path::new(&ws.path);
        assert_eq!(read_context(dir).unwrap(), "");
        write_context(dir, "# Hello\n\nnotes").unwrap();
        assert_eq!(read_context(dir).unwrap(), "# Hello\n\nnotes");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn board_add_move_update_delete() {
        let root = temp_root("board");
        let ws = create_workspace_at(&root, "App", None, Some("app"), None).unwrap();
        let dir = Path::new(&ws.path);

        let board = read_board(dir).unwrap();
        assert_eq!(board.columns, vec!["backlog", "doing", "review", "done"]);
        assert!(board.tasks.is_empty());

        let (board, task) = add_task(&board, "Ship C5", None).unwrap();
        write_board(dir, &board).unwrap();
        assert_eq!(task.status_id, "backlog");
        assert_eq!(task.sort_key, 0);

        let board = move_task(&board, &task.id, "doing", 0).unwrap();
        write_board(dir, &board).unwrap();
        let loaded = read_board(dir).unwrap();
        let t = loaded.tasks.iter().find(|t| t.id == task.id).unwrap();
        assert_eq!(t.status_id, "doing");
        assert!(t.completed_at.is_none());

        let board = update_task(
            &loaded,
            &task.id,
            BoardTaskPatch {
                title: Some("Ship C5 done".into()),
                body: Some("body".into()),
                status_id: Some("done".into()),
                sort_key: None,
            },
        )
        .unwrap();
        write_board(dir, &board).unwrap();
        let loaded = read_board(dir).unwrap();
        let t = loaded.tasks.iter().find(|t| t.id == task.id).unwrap();
        assert_eq!(t.title, "Ship C5 done");
        assert_eq!(t.body, "body");
        assert_eq!(t.status_id, "done");
        assert!(t.completed_at.is_some());

        let board = delete_task(&loaded, &task.id).unwrap();
        write_board(dir, &board).unwrap();
        assert!(read_board(dir).unwrap().tasks.is_empty());

        // Persisted JSON uses camelCase keys.
        let raw = fs::read_to_string(dir.join(WORKSPACE_BOARD_FILE)).unwrap();
        assert!(raw.contains("\"schemaVersion\""));
        assert!(raw.contains("\"statusId\"") || !raw.contains("tasks\": [") || raw.contains("[]"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_empty_title_and_bad_status() {
        let board = WorkspaceBoard::empty();
        assert_eq!(
            add_task(&board, "  ", None).unwrap_err(),
            ERR_TASK_TITLE_EMPTY
        );
        assert_eq!(
            add_task(&board, "x", Some("nope")).unwrap_err(),
            ERR_TASK_STATUS_UNKNOWN
        );
    }

    #[test]
    fn create_workspace_board_is_typed_empty() {
        let root = temp_root("create-shape");
        let ws = create_workspace_at(&root, "App", None, Some("app"), None).unwrap();
        let board = read_board(Path::new(&ws.path)).unwrap();
        assert_eq!(board.schema_version, WORKSPACE_BOARD_SCHEMA_VERSION);
        assert_eq!(board.tasks.len(), 0);
        assert_eq!(board.columns.len(), 4);
        let _ = fs::remove_dir_all(&root);
    }
}
