//! 托管 sidecar 的端口回退（ADR 0022）。
//!
//! 背景：三个后端都按 `record.base_url`（managed 权威）绑一个固定回环端口
//! （tool-runtime 18778 / file-gateway 18787 / knowledge 18790）。若该端口被**非本服务**
//! 的进程占用，spawn 出的子进程 bind 失败 → 静默崩溃 → AFM 反复探活失败。
//!
//! 策略（仅用于"我们要 spawn"的分支；若 `record.base_url` 探活健康则是附着、绝不改端口）：
//! 目标端口若不可绑（被占）→ 从 `want+1` 起顺延，找第一个可绑端口，写回 `record.base_url`，
//! 并把最终端口经 pack 声明的 `port_env` 注入子进程。客户端（file/knowledge）由命令层把
//! 生效 URL 推入其既有 `CONFIGURED_URL` 全局，tool-runtime 客户端本就读 record → 三处一致。

use std::net::{IpAddr, TcpListener};

/// 解析 `scheme://host:port` 形式的 base URL，返回 `(前缀 "scheme://host", 端口)`。
/// 仅处理托管实际使用的形态；无法解析返回 `None`（调用方按"不改端口"处理）。
pub fn split_base_url(url: &str) -> Option<(String, u16)> {
    let scheme_end = url.find("://")?;
    let (scheme, rest) = url.split_at(scheme_end);
    let rest = &rest[3..]; // 跳过 "://"
                           // 取 host[:port]，截到第一个 '/'（base_url 已规范化无尾斜杠，但防御性处理）。
    let hostport = rest.split('/').next().unwrap_or("");
    let (host, port_str) = if let Some(stripped) = hostport.strip_prefix('[') {
        // IPv6：[::1]:18787
        let close = stripped.find(']')?;
        let host = format!("[{}]", &stripped[..close]); // 补回中括号
        let tail = &stripped[close + 1..];
        (host, tail.strip_prefix(':')?.to_string())
    } else {
        let colon = hostport.rfind(':')?;
        (
            hostport[..colon].to_string(),
            hostport[colon + 1..].to_string(),
        )
    };
    let port: u16 = port_str.parse().ok()?;
    Some((format!("{scheme}://{host}"), port))
}

/// 取 base URL 的端口（无则 `None`）。
pub fn port_of(url: &str) -> Option<u16> {
    split_base_url(url).map(|(_, p)| p)
}

/// 该端口此刻是否可绑（绑定成功=空闲）。仅对回环地址尝试，避免误碰公网/其它网卡。
pub fn is_bindable(host: &str, port: u16) -> bool {
    let host = host.trim_matches(|c| c == '[' || c == ']');
    let ip: IpAddr = match host.parse() {
        Ok(ip) => ip,
        Err(_) => return false,
    };
    TcpListener::bind((ip, port)).is_ok()
}

/// 若 `url` 的端口已被占用，从 `want+1` 起在 `span` 范围内找第一个可绑端口并返回替换后的 URL；
/// 端口本就空闲、或范围内都不可绑 → 原样返回。永不 panic、永不阻塞（纯本地 bind 探测）。
pub fn relocate_if_occupied(url: &str, span: u16) -> String {
    let (prefix, want) = match split_base_url(url) {
        Some(v) => v,
        None => return url.to_string(),
    };
    // 前缀里去掉 scheme，拿 host 做 bind 探测。
    let host = prefix
        .split_once("://")
        .map(|(_, h)| h)
        .unwrap_or("127.0.0.1");
    if is_bindable(host, want) {
        return url.to_string(); // 空闲：用默认端口。
    }
    for off in 1..=span {
        let candidate = match want.checked_add(off) {
            Some(p) => p,
            None => break,
        };
        if is_bindable(host, candidate) {
            return format!("{prefix}:{candidate}");
        }
    }
    url.to_string() // 全占：交回子进程自行处理（不静默改到不可知端口）。
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_ipv4_base_url() {
        assert_eq!(
            split_base_url("http://127.0.0.1:18787"),
            Some(("http://127.0.0.1".to_string(), 18787))
        );
        assert_eq!(port_of("http://127.0.0.1:18790"), Some(18790));
    }

    #[test]
    fn splits_ipv6_base_url() {
        assert_eq!(
            split_base_url("http://[::1]:18778"),
            Some(("http://[::1]".to_string(), 18778))
        );
    }

    #[test]
    fn unparseable_url_returns_none() {
        assert_eq!(split_base_url("not a url"), None);
        assert_eq!(split_base_url("http://127.0.0.1:notaport"), None);
        assert_eq!(port_of("http://127.0.0.1"), None);
    }

    #[test]
    fn relocate_is_noop_when_port_free() {
        // 让 OS 分配一个空闲端口，确认 relocate 不改它。
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let free = l.local_addr().unwrap().port();
        drop(l);
        let url = format!("http://127.0.0.1:{free}");
        assert_eq!(relocate_if_occupied(&url, 16), url);
    }

    #[test]
    fn relocate_moves_off_an_occupied_port() {
        let held = TcpListener::bind("127.0.0.1:0").unwrap();
        let busy = held.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{busy}");
        let moved = relocate_if_occupied(&url, 64);
        assert_ne!(moved, url, "occupied port must relocate");
        let new_port = port_of(&moved).unwrap();
        assert!(new_port > busy && new_port <= busy + 64);
        assert!(
            is_bindable("127.0.0.1", new_port),
            "relocated port must be free"
        );
        drop(held);
    }
}
