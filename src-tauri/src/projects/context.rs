// SPDX-License-Identifier: GPL-3.0-or-later
//! Workspace `CONTEXT.md` read / write (AIA-211 / C5).
//!
//! D5: the old `board.json` four-column task board is retired — tasks now live only as a
//! plain `tasks.json` file (Backlog.md fields) written at project create and edited by AI /
//! hand. This module keeps just the CONTEXT.md persistence that the Workspace commands use.

use super::workspace::{load_workspace, WORKSPACE_CONTEXT_FILE};
use std::fs;
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::workspace::create_workspace_at;
    use std::path::PathBuf;

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-context-{label}-{}",
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
}
