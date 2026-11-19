// SPDX-License-Identifier: GPL-3.0-or-later
//! 声明版兼容门（ADR 0019 / 0020，Q15=a / Q19）：managed spawn 前用**元清单声明的产物版本**
//! 落进 pack 的 `versionCompat` 区间校验，复用 `tool_runtime::semver_compat` 的三段数字比较。
//! 语义对齐既有 `assert_gateway_compat`：无区间 / 未知或缺省版本一律放行（不拦截），仅对能解析
//! 为三段且越界的版本拒绝。config-mode 只附着已有实例，**不经此门**。

use crate::tool_runtime::{known_triple, version_in_range};

/// `versionCompat` 的 `[min, max]` 闭区间（本地中性结构，不绑定 `GatewayCompat`，便于字段更名）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRange {
    pub min: String,
    pub max: String,
}

/// 校验声明版产物是否落在兼容区间内；不匹配返回带说明的 `Err`。
pub fn ensure_declared_version_compat(
    artifact_version: &str,
    range: Option<&VersionRange>,
) -> Result<(), String> {
    let Some(range) = range else {
        return Ok(()); // 未声明区间 → 不拦。
    };
    let version = artifact_version.trim();
    if version.is_empty() || !known_triple(version) {
        return Ok(()); // 未知/缺省版本不参与硬拦。
    }
    if version_in_range(version, &range.min, &range.max) {
        Ok(())
    } else {
        Err(format!(
            "versionCompat mismatch: artifact version {version} not in [{}, {}]",
            range.min, range.max
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(min: &str, max: &str) -> VersionRange {
        VersionRange { min: min.to_string(), max: max.to_string() }
    }

    #[test]
    fn no_range_allows_anything() {
        assert!(ensure_declared_version_compat("9.9.9", None).is_ok());
        assert!(ensure_declared_version_compat("", None).is_ok());
    }

    #[test]
    fn unknown_or_empty_version_skips_gate() {
        let r = range("0.1.0", "1.0.0");
        assert!(ensure_declared_version_compat("", Some(&r)).is_ok());
        assert!(ensure_declared_version_compat("  ", Some(&r)).is_ok());
        assert!(ensure_declared_version_compat("dev", Some(&r)).is_ok());
    }

    #[test]
    fn in_range_passes_out_of_range_rejects() {
        let r = range("0.1.0", "1.0.0");
        assert!(ensure_declared_version_compat("0.4.2", Some(&r)).is_ok());
        assert!(ensure_declared_version_compat("0.1.0", Some(&r)).is_ok());
        assert!(ensure_declared_version_compat("1.0.0", Some(&r)).is_ok());
        let err = ensure_declared_version_compat("2.0.0", Some(&r)).unwrap_err();
        assert!(err.contains("versionCompat mismatch"), "{err}");
        assert!(err.contains("2.0.0"), "{err}");
    }
}
