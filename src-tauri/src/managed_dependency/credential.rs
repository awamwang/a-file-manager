// SPDX-License-Identifier: GPL-3.0-or-later
//! 托管依赖 credential 来源解析（ADR 0020）：把 file / tool-manager / knowledge 的凭据差异
//! 统一到一个解析入口。`read_file` / `env` 注入，单测不触真实文件与环境变量。

use super::registry::CredentialSource;
use std::path::Path;

/// 解析 `CredentialSource` 为可选 token 字符串。
///
/// - `None` → `Ok(None)`（如 awam-file）。
/// - `EnvVar`：取到且非空 → `Some(trimmed)`；未设/空 → `Ok(None)`（env 视为可缺省）。
/// - `TokenFile`：读文件去空白；空内容或读失败 → `Err`（显式配置的文件缺失是真错误）。
pub fn resolve_credential<R, E>(
    source: &CredentialSource,
    read_file: R,
    env: E,
) -> Result<Option<String>, String>
where
    R: FnOnce(&Path) -> Result<String, String>,
    E: Fn(&str) -> Option<String>,
{
    match source {
        CredentialSource::None => Ok(None),
        CredentialSource::EnvVar { name } => Ok(env(name)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())),
        CredentialSource::TokenFile { path } => {
            let text = read_file(Path::new(path))?;
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                return Err(format!("credential token file empty: {path}"));
            }
            Ok(Some(trimmed))
        }
    }
}

/// 生产解析：真实读文件 + 真实环境变量。
#[allow(dead_code)] // 接线于 Task 5 的 managed 调用路径。
pub fn resolve_credential_native(source: &CredentialSource) -> Result<Option<String>, String> {
    resolve_credential(
        source,
        |path| {
            std::fs::read_to_string(path)
                .map_err(|e| format!("read credential {}: {e}", path.display()))
        },
        |key| std::env::var(key).ok(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_map(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    #[test]
    fn none_yields_no_credential() {
        assert_eq!(resolve_credential(&CredentialSource::None, |_| Err("no".into()), env_map(&[])).unwrap(), None);
    }

    #[test]
    fn env_var_present_is_trimmed_and_absent_is_none() {
        let src = CredentialSource::EnvVar { name: "AWAM_HTTP_TOKEN".to_string() };
        let got = resolve_credential(&src, |_| Err("no".into()), env_map(&[("AWAM_HTTP_TOKEN", " secret \n")])).unwrap();
        assert_eq!(got.as_deref(), Some("secret"));
        // 未设该 env → None（可缺省）。
        let missing = resolve_credential(&src, |_| Err("no".into()), env_map(&[])).unwrap();
        assert_eq!(missing, None);
        // 设了但空白 → 视作无。
        let blank = resolve_credential(&src, |_| Err("no".into()), env_map(&[("AWAM_HTTP_TOKEN", "   ")])).unwrap();
        assert_eq!(blank, None);
    }

    #[test]
    fn token_file_reads_and_trims() {
        let src = CredentialSource::TokenFile { path: "C:/tok/http-token".to_string() };
        let got = resolve_credential(&src, |_| Ok("file-token\r\n".to_string()), env_map(&[])).unwrap();
        assert_eq!(got.as_deref(), Some("file-token"));
    }

    #[test]
    fn token_file_empty_or_unreadable_is_error() {
        let src = CredentialSource::TokenFile { path: "C:/tok/http-token".to_string() };
        let empty = resolve_credential(&src, |_| Ok("   \n".to_string()), env_map(&[]));
        assert!(empty.unwrap_err().contains("token file empty"));
        let unreadable = resolve_credential(&src, |_| Err("os error 2".to_string()), env_map(&[]));
        assert!(unreadable.unwrap_err().contains("os error 2"));
    }
}
