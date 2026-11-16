// SPDX-License-Identifier: GPL-3.0-or-later

use super::sync::default_document;
use super::types::UserWorkGroups;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

pub const WORK_GROUPS_FILE_NAME: &str = "work-groups.json";

/// FIL-68：pin 与 openState 同住 work-groups.json；串行化所有 load→save，
/// 避免并发写同一文件互相覆盖（lost update）。命令均为同步 fn，阻塞安全。
static DOC_LOCK: Mutex<()> = Mutex::new(());

/// 跨模块（work_groups / projects）共享的文档写锁。
pub fn lock_document() -> Result<MutexGuard<'static, ()>, String> {
    DOC_LOCK
        .lock()
        .map_err(|_| "work-groups doc lock poisoned".to_string())
}

pub fn work_groups_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(WORK_GROUPS_FILE_NAME)
}

pub fn load_from_path(config_dir: &Path) -> Result<UserWorkGroups, String> {
    let path = work_groups_file_path(config_dir);
    if !path.exists() {
        let document = default_document();
        save_to_path(config_dir, &document)?;
        return Ok(document);
    }
    let text =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read work groups: {err}"))?;
    let mut document: UserWorkGroups = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse work groups JSON: {err}"))?;
    if document.work_groups.is_empty() {
        document = default_document();
        save_to_path(config_dir, &document)?;
    }
    if !document
        .work_groups
        .iter()
        .any(|group| group.id == document.active_work_group_id)
    {
        document.active_work_group_id = document.work_groups[0].id.clone();
        save_to_path(config_dir, &document)?;
    }
    Ok(document)
}

pub fn save_to_path(config_dir: &Path, document: &UserWorkGroups) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create work groups directory: {err}"))?;
    let path = work_groups_file_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize work groups: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("Failed to write work groups: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_groups::sync::create_work_group_in_list;

    #[test]
    fn persists_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-wg-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut document = load_from_path(&dir).unwrap();
        let (groups, created) =
            create_work_group_in_list(&document.work_groups, Some("Focus".into()));
        document.work_groups = groups;
        document.active_work_group_id = created.id.clone();
        save_to_path(&dir, &document).unwrap();

        let loaded = load_from_path(&dir).unwrap();
        assert_eq!(loaded.active_work_group_id, created.id);
        assert!(loaded.work_groups.iter().any(|g| g.name == "Focus"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pinned_tab_round_trip_keeps_wire_shape() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-wg-pin-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut document = load_from_path(&dir).unwrap();
        document.pinned_tab = Some(crate::work_groups::types::PinnedTab {
            path: r"D:\pinned".to_string(),
            name: "pinned".to_string(),
            tab_type: "directory".to_string(),
            strip_index: 1,
        });
        save_to_path(&dir, &document).unwrap();

        // 落盘 wire 形状：顶层 pinnedTab + camelCase + type 改名。
        let raw = fs::read_to_string(work_groups_file_path(&dir)).unwrap();
        assert!(raw.contains("\"pinnedTab\""), "missing pinnedTab key: {raw}");
        assert!(raw.contains("\"stripIndex\": 1"), "missing stripIndex: {raw}");
        assert!(raw.contains("\"type\": \"directory\""), "missing type rename: {raw}");

        let loaded = load_from_path(&dir).unwrap();
        assert_eq!(loaded.pinned_tab, document.pinned_tab);

        // 旧文件缺 pinnedTab 字段（#[serde(default)]）仍可解析为 None。
        let legacy = UserWorkGroups {
            work_groups: document.work_groups.clone(),
            active_work_group_id: document.active_work_group_id.clone(),
            pinned_tab: None,
        };
        let legacy_json = serde_json::to_string(&legacy).unwrap();
        let manual: UserWorkGroups =
            serde_json::from_str(&legacy_json.replace(",\"pinnedTab\":null", "")).unwrap();
        assert!(manual.pinned_tab.is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
