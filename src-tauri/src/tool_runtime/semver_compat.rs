// SPDX-License-Identifier: GPL-3.0-or-later

/// 判断 `version` 是否在 `[min, max]` 闭区间内（仅比较主.次.补丁三段数字）。
pub fn version_in_range(version: &str, min: &str, max: &str) -> bool {
    let Some(v) = parse_triple(version) else {
        return false;
    };
    let Some(lo) = parse_triple(min) else {
        return false;
    };
    let Some(hi) = parse_triple(max) else {
        return false;
    };
    v >= lo && v <= hi
}

fn parse_triple(s: &str) -> Option<(u32, u32, u32)> {
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_in_range_inclusive() {
        assert!(version_in_range("1.2.0", "1.0.0", "2.0.0"));
        assert!(!version_in_range("2.1.0", "1.0.0", "2.0.0"));
    }
}
