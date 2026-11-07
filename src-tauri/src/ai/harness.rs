// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit Harness / Superpower init (AIA-219 / A6 · ADR 0008).
//!
//! Writes `AGENTS.md` + `skills/` (may be empty). Optional SDD (default off):
//! `docs/superpowers/{specs,plans}/` and `.superpowers/sdd/progress.md`.
//! Never part of Core Project create.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const ERR_HARNESS_WORKSPACE: &str = "AI_HARNESS_WORKSPACE_INVALID";

const AGENTS_FILE: &str = "AGENTS.md";
const SKILLS_DIR: &str = "skills";
const CONTEXT_FILE: &str = "CONTEXT.md";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HarnessInitReport {
    pub workspace_path: String,
    pub include_sdd: bool,
    pub written: Vec<String>,
    pub skipped: Vec<String>,
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

fn write_new(path: &Path, contents: &str, written: &mut Vec<String>, skipped: &mut Vec<String>) {
    if path.exists() {
        skipped.push(normalize(path));
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(path, contents).is_ok() {
        written.push(normalize(path));
    } else {
        skipped.push(normalize(path));
    }
}

fn ensure_dir(path: &Path, written: &mut Vec<String>, skipped: &mut Vec<String>) {
    if path.is_dir() {
        skipped.push(normalize(path));
        return;
    }
    match fs::create_dir_all(path) {
        Ok(()) => written.push(normalize(path)),
        Err(_) => skipped.push(normalize(path)),
    }
}

fn agents_template(display_name: &str) -> String {
    format!(
        "# AGENTS.md — {display_name}\n\n\
         This file was created by AFM AI Harness init.\n\n\
         ## Skills\n\n\
         Project skills live under `./skills/` (may be empty). Prefer this over editor-local skill folders.\n\n\
         ## Context\n\n\
         See `CONTEXT.md` for project context.\n"
    )
}

fn sdd_progress_template() -> &'static str {
    "# SDD progress\n\nInitialized by AFM AI Harness (SDD option).\n\n- [ ] Spec\n- [ ] Plan\n- [ ] Implement\n"
}

/// Explicit Harness init on an existing Project Workspace directory.
pub fn init_harness(
    workspace_path: &str,
    include_sdd: bool,
    reinforce_context: bool,
) -> Result<HarnessInitReport, String> {
    let root = PathBuf::from(workspace_path.trim());
    if !root.is_dir() || !root.join("project.json").is_file() {
        return Err(ERR_HARNESS_WORKSPACE.to_string());
    }

    let display_name = fs::read_to_string(root.join("project.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| {
            v.get("displayName")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| {
            root.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Project")
                .to_string()
        });

    let mut written = Vec::new();
    let mut skipped = Vec::new();

    write_new(
        &root.join(AGENTS_FILE),
        &agents_template(&display_name),
        &mut written,
        &mut skipped,
    );
    ensure_dir(&root.join(SKILLS_DIR), &mut written, &mut skipped);

    if reinforce_context {
        let ctx = root.join(CONTEXT_FILE);
        if ctx.is_file() {
            let existing = fs::read_to_string(&ctx).unwrap_or_default();
            let marker = "<!-- afm-harness -->";
            if !existing.contains(marker) {
                let appendix = format!(
                    "\n\n{marker}\n## Agent harness\n\nInitialized. See `AGENTS.md` and `skills/`.\n"
                );
                let _ = fs::write(&ctx, format!("{existing}{appendix}"));
                written.push(normalize(&ctx));
            } else {
                skipped.push(normalize(&ctx));
            }
        }
    }

    if include_sdd {
        ensure_dir(
            &root.join("docs").join("superpowers").join("specs"),
            &mut written,
            &mut skipped,
        );
        ensure_dir(
            &root.join("docs").join("superpowers").join("plans"),
            &mut written,
            &mut skipped,
        );
        write_new(
            &root.join(".superpowers").join("sdd").join("progress.md"),
            sdd_progress_template(),
            &mut written,
            &mut skipped,
        );
    }

    Ok(HarnessInitReport {
        workspace_path: normalize(&root),
        include_sdd,
        written,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::workspace::create_workspace_at;

    #[test]
    fn harness_writes_agents_and_skills_not_sdd_by_default() {
        let root = std::env::temp_dir().join(format!(
            "afm-harness-{}-{}",
            "basic",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let created = create_workspace_at(&root, "Demo", None, Some("demo"), None).unwrap();
        let ws = PathBuf::from(&created.path);
        assert!(!ws.join("AGENTS.md").exists());

        let report = init_harness(&created.path, false, true).unwrap();
        assert!(ws.join("AGENTS.md").is_file());
        assert!(ws.join("skills").is_dir());
        assert!(!ws.join(".superpowers").exists());
        assert!(report.written.iter().any(|p| p.ends_with("AGENTS.md")));

        // Idempotent skip
        let again = init_harness(&created.path, false, true).unwrap();
        assert!(again.skipped.iter().any(|p| p.ends_with("AGENTS.md")));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sdd_option_writes_superpowers_tree() {
        let root = std::env::temp_dir().join(format!(
            "afm-harness-{}-{}",
            "sdd",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let created = create_workspace_at(&root, "Sdd", None, Some("sdd"), None).unwrap();
        init_harness(&created.path, true, false).unwrap();
        let ws = PathBuf::from(&created.path);
        assert!(ws.join("docs").join("superpowers").join("specs").is_dir());
        assert!(ws.join(".superpowers").join("sdd").join("progress.md").is_file());
        let _ = fs::remove_dir_all(&root);
    }
}
