// SPDX-License-Identifier: GPL-3.0-or-later
//! Skill Junction deploy to enabled Agent Targets (AIA-217 / A4 · ADR 0005).
//!
//! Deploys library skills into each **enabled** Agent Target's global Skills dir
//! (and optionally project-relative dir under a Workspace). Junction only — never
//! copies. Removing undeploy unlinks the junction; library skill stays.

use super::agents::{expand_agent_path, AgentTarget, AiConfigDocument};
use super::skills::{
    ensure_library_root, list_skills, resolve_skills_library_root, validate_skill_name_pub,
    LibrarySkill, ERR_SKILL_UNKNOWN,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const ERR_DEPLOY_NO_ENABLED: &str = "AI_DEPLOY_NO_ENABLED";
pub const ERR_DEPLOY_TARGET_CONFLICT: &str = "AI_DEPLOY_TARGET_CONFLICT";
pub const ERR_DEPLOY_PROJECT_REQUIRED: &str = "AI_DEPLOY_PROJECT_REQUIRED";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillDeployment {
    pub skill_name: String,
    pub agent_id: String,
    pub scope: String, // "global" | "project"
    pub junction_path: String,
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeployReport {
    pub skill_name: String,
    pub deployments: Vec<SkillDeployment>,
    pub skipped: Vec<String>,
}

fn normalize_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

fn create_directory_junction(link: &Path, target: &Path) -> Result<(), String> {
    if link.exists() {
        return Err(ERR_DEPLOY_TARGET_CONFLICT.to_string());
    }
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to ensure skills dir: {err}"))?;
    }
    let link_s = normalize_path_string(link);
    let target_s = fs::canonicalize(target)
        .map(|p| normalize_path_string(&p))
        .unwrap_or_else(|_| normalize_path_string(target));
    let target_s = target_s
        .strip_prefix(r"\\?\")
        .unwrap_or(&target_s)
        .to_string();

    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J", &link_s, &target_s])
        .output()
        .map_err(|err| format!("Failed to run mklink: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Failed to create skill junction: {} {}",
            stdout.trim(),
            stderr.trim()
        ));
    }
    Ok(())
}

fn remove_junction(link: &Path) -> Result<(), String> {
    if !link.exists() {
        return Ok(());
    }
    fs::remove_dir(link).map_err(|err| format!("Failed to remove skill junction: {err}"))
}

fn skill_in_library(
    config_dir: &Path,
    document: &AiConfigDocument,
    name: &str,
) -> Result<LibrarySkill, String> {
    let name = validate_skill_name_pub(name)?;
    let root = resolve_skills_library_root(config_dir, document);
    ensure_library_root(&root)?;
    list_skills(&root)?
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| ERR_SKILL_UNKNOWN.to_string())
}

fn enabled_agents(document: &AiConfigDocument) -> Vec<&AgentTarget> {
    document
        .agent_targets
        .iter()
        .filter(|a| a.enabled)
        .collect()
}

fn deploy_one(
    agent: &AgentTarget,
    skill: &LibrarySkill,
    skills_dir_raw: &str,
    scope: &str,
) -> Result<Option<SkillDeployment>, String> {
    let expanded = expand_agent_path(skills_dir_raw);
    if expanded.is_empty() {
        return Ok(None);
    }
    let dir = PathBuf::from(&expanded);
    fs::create_dir_all(&dir).map_err(|err| format!("Failed to ensure agent skills dir: {err}"))?;
    let link = dir.join(&skill.name);
    if link.exists() {
        // Idempotent if already a reparse pointing at same target.
        if let Ok(meta) = fs::symlink_metadata(&link) {
            if meta.file_type().is_symlink() {
                if let Ok(current) = fs::read_link(&link) {
                    let cur = normalize_path_string(&current).to_ascii_lowercase();
                    let want = normalize_path_string(Path::new(&skill.path)).to_ascii_lowercase();
                    if cur == want
                        || cur.ends_with(&format!("\\{}", skill.name.to_ascii_lowercase()))
                    {
                        return Ok(Some(SkillDeployment {
                            skill_name: skill.name.clone(),
                            agent_id: agent.id.clone(),
                            scope: scope.to_string(),
                            junction_path: normalize_path_string(&link),
                            target_path: skill.path.clone(),
                        }));
                    }
                }
            }
        }
        return Err(format!(
            "{ERR_DEPLOY_TARGET_CONFLICT}: {} already exists",
            normalize_path_string(&link)
        ));
    }
    create_directory_junction(&link, Path::new(&skill.path))?;
    Ok(Some(SkillDeployment {
        skill_name: skill.name.clone(),
        agent_id: agent.id.clone(),
        scope: scope.to_string(),
        junction_path: normalize_path_string(&link),
        target_path: skill.path.clone(),
    }))
}

/// Deploy one library skill to all enabled Agent Targets (global dirs).
/// When `project_workspace` is set, also deploy into each agent's project-relative skills dir.
pub fn deploy_skill(
    config_dir: &Path,
    document: &AiConfigDocument,
    skill_name: &str,
    project_workspace: Option<&str>,
) -> Result<DeployReport, String> {
    let skill = skill_in_library(config_dir, document, skill_name)?;
    let agents = enabled_agents(document);
    if agents.is_empty() {
        return Err(ERR_DEPLOY_NO_ENABLED.to_string());
    }

    let mut deployments = Vec::new();
    let mut skipped = Vec::new();

    for agent in agents {
        match deploy_one(agent, &skill, &agent.global_skills_dir, "global") {
            Ok(Some(d)) => deployments.push(d),
            Ok(None) => skipped.push(format!("{}: empty global skills dir", agent.id)),
            Err(err) => skipped.push(format!("{}: {err}", agent.id)),
        }

        if let Some(ws) = project_workspace {
            let ws_path = PathBuf::from(ws.trim());
            if !ws_path.is_dir() {
                return Err(ERR_DEPLOY_PROJECT_REQUIRED.to_string());
            }
            let rel = agent.project_skills_dir.trim();
            if rel.is_empty() {
                skipped.push(format!("{}: empty project skills dir", agent.id));
                continue;
            }
            let project_skills = ws_path.join(rel.replace('/', "\\"));
            let raw = normalize_path_string(&project_skills);
            match deploy_one(agent, &skill, &raw, "project") {
                Ok(Some(d)) => deployments.push(d),
                Ok(None) => {}
                Err(err) => skipped.push(format!("{}@project: {err}", agent.id)),
            }
        }
    }

    Ok(DeployReport {
        skill_name: skill.name,
        deployments,
        skipped,
    })
}

pub fn undeploy_skill(
    config_dir: &Path,
    document: &AiConfigDocument,
    skill_name: &str,
    project_workspace: Option<&str>,
) -> Result<DeployReport, String> {
    let skill = skill_in_library(config_dir, document, skill_name)?;
    let agents = enabled_agents(document);
    let mut deployments = Vec::new();
    let mut skipped = Vec::new();

    for agent in &agents {
        let global = expand_agent_path(&agent.global_skills_dir);
        if !global.is_empty() {
            let link = PathBuf::from(&global).join(&skill.name);
            if link.exists() {
                remove_junction(&link)?;
                deployments.push(SkillDeployment {
                    skill_name: skill.name.clone(),
                    agent_id: agent.id.clone(),
                    scope: "global".into(),
                    junction_path: normalize_path_string(&link),
                    target_path: skill.path.clone(),
                });
            }
        }
        if let Some(ws) = project_workspace {
            let rel = agent.project_skills_dir.trim();
            if rel.is_empty() {
                continue;
            }
            let link = PathBuf::from(ws).join(rel.replace('/', "\\")).join(&skill.name);
            if link.exists() {
                remove_junction(&link)?;
                deployments.push(SkillDeployment {
                    skill_name: skill.name.clone(),
                    agent_id: agent.id.clone(),
                    scope: "project".into(),
                    junction_path: normalize_path_string(&link),
                    target_path: skill.path.clone(),
                });
            }
        }
    }

    if agents.is_empty() {
        skipped.push(ERR_DEPLOY_NO_ENABLED.to_string());
    }

    Ok(DeployReport {
        skill_name: skill.name,
        deployments,
        skipped,
    })
}

/// Scan a Project Workspace for folders that look like skills (`SKILL.md`) not yet in library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSkillCandidate {
    pub name: String,
    pub path: String,
}

pub fn search_project_skills(workspace: &str) -> Result<Vec<ProjectSkillCandidate>, String> {
    let root = PathBuf::from(workspace.trim());
    if !root.is_dir() {
        return Err(ERR_DEPLOY_PROJECT_REQUIRED.to_string());
    }
    let mut out = Vec::new();
    // Shallow + one-level common dirs (skills/, .cursor/skills, etc.)
    let mut roots = vec![root.clone()];
    for child in ["skills", ".cursor\\skills", ".agents\\skills", ".claude\\skills"] {
        let p = root.join(child);
        if p.is_dir() {
            roots.push(p);
        }
    }
    for dir in roots {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if path.join("SKILL.md").is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                out.push(ProjectSkillCandidate {
                    name,
                    path: normalize_path_string(&path),
                });
            }
        }
        // Also if the dir itself is a skill
        if dir.join("SKILL.md").is_file() {
            if let Some(name) = dir.file_name().and_then(|s| s.to_str()) {
                if !out.iter().any(|c| c.path == normalize_path_string(&dir)) {
                    out.push(ProjectSkillCandidate {
                        name: name.to_string(),
                        path: normalize_path_string(&dir),
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    out.dedup_by(|a, b| a.path == b.path);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agents::{load_from_path, set_enabled, save_to_path};
    use crate::ai::skills::import_skill;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-ai-deploy-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn deploy_and_undeploy_junction_to_enabled_agent() {
        let dir = temp_dir("junc");
        let mut doc = load_from_path(&dir).unwrap();
        // Point a custom agent at a temp skills dir and enable it.
        let agent_skills = dir.join("agent-skills");
        fs::create_dir_all(&agent_skills).unwrap();
        doc = crate::ai::agents::upsert_custom(
            &doc,
            Some("test-agent".into()),
            "Test Agent".into(),
            agent_skills.to_string_lossy().into(),
            r".skills".into(),
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();
        doc = set_enabled(&doc, "test-agent", true).unwrap();
        save_to_path(&dir, &doc).unwrap();

        let src = dir.join("src-skill");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("SKILL.md"), "# t\n").unwrap();
        let (_, skill) = import_skill(&dir, &doc, src.to_str().unwrap(), Some("demo".into())).unwrap();

        let report = deploy_skill(&dir, &doc, "demo", None).unwrap();
        assert_eq!(report.skill_name, "demo");
        assert!(report.deployments.iter().any(|d| d.agent_id == "test-agent"));
        let link = agent_skills.join("demo");
        assert!(link.exists());
        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink());
        // Library still present
        assert!(Path::new(&skill.path).join("SKILL.md").is_file());

        let und = undeploy_skill(&dir, &doc, "demo", None).unwrap();
        assert!(und.deployments.iter().any(|d| d.agent_id == "test-agent"));
        assert!(!link.exists());
        assert!(Path::new(&skill.path).join("SKILL.md").is_file());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_finds_project_skill_folders() {
        let dir = temp_dir("search");
        let skill = dir.join("skills").join("local");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "# local\n").unwrap();
        let found = search_project_skills(dir.to_str().unwrap()).unwrap();
        assert!(found.iter().any(|c| c.name == "local"));
        let _ = fs::remove_dir_all(&dir);
    }
}
