//! 输出解析工具函数。

/// 把 `docker ps` 的 ports 字段（形如 "0.0.0.0:8080->80/tcp, :::8080->80/tcp"）
/// 拆成人类可读的端口映射列表。
pub fn parse_ports(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
