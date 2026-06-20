//! docker logs 命令构造。
//!
//! 实际的流式读取在 commands 层通过 SshClient::exec_stream 完成，
//! 这里只负责按参数拼出正确的命令字符串。

/// 构造 `docker logs` 命令。
///
/// - `follow=true`  → `-f` 实时跟踪
/// - `tail="all"`   → 不限制；传数字则只看最后 N 行
/// - `since`/`until` → 可选时间戳过滤（RFC3339 或 unix 相对时间，如 `2024-01-01T00:00:00`、`10m`）
/// - `timestamps`   → 每行加时间戳前缀
pub fn build_logs_cmd(
    id_or_name: &str,
    follow: bool,
    tail: &str,
    since: Option<&str>,
    until: Option<&str>,
    timestamps: bool,
) -> String {
    let mut parts = vec!["docker logs".to_string()];
    if follow {
        parts.push("--follow".into());
    }
    if timestamps {
        parts.push("--timestamps".into());
    }
    if let Some(s) = since {
        if !s.is_empty() {
            parts.push(format!("--since {s}"));
        }
    }
    if let Some(u) = until {
        if !u.is_empty() {
            parts.push(format!("--until {u}"));
        }
    }
    parts.push(format!("--tail {tail}"));
    parts.push(id_or_name.to_string());
    parts.join(" ")
}
