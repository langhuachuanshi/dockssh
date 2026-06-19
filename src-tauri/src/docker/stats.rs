//! docker stats 解析。
//!
//! `docker stats` 默认输出带对齐的彩色表格，难以稳定解析。
//! 用 `--format '{{json .}}'` 让每行输出一个 JSON 对象，
//! 每 N 秒一行，持续输出 → 适合流式读取（commands 层 exec_stream）。

use serde::Deserialize;

use crate::error::AppResult;
use crate::models::StatsSample;
use crate::ssh::client::SshClient;

#[derive(Debug, Deserialize)]
struct StatsRow {
    #[serde(rename = "Container", default)]
    container: String,
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "CPUPerc", default)]
    cpu: String,
    #[serde(rename = "MemUsage", default)]
    mem_usage: String,
    #[serde(rename = "MemPerc", default)]
    mem: String,
    #[serde(rename = "NetIO", default)]
    net: String,
    #[serde(rename = "BlockIO", default)]
    block: String,
    #[serde(rename = "PIDs", default)]
    pids: String,
}

/// 把 "12.34%" 解析成 12.34。
fn pct(s: &str) -> f64 {
    s.trim()
        .trim_end_matches('%')
        .parse::<f64>()
        .unwrap_or(0.0)
}

/// 把 "PIDs" 形如 "32" 解析成 u64。
fn pids(s: &str) -> u64 {
    s.trim().parse::<u64>().unwrap_or(0)
}

/// 从一行 stats JSON 文本解析成采样。
pub fn parse_line(line: &str) -> Option<StatsSample> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let row: StatsRow = serde_json::from_str(line).ok()?;
    Some(StatsSample {
        container_id: row.container,
        name: row.name,
        cpu_percent: pct(&row.cpu),
        mem_usage: row.mem_usage,
        mem_percent: pct(&row.mem),
        net_io: row.net,
        block_io: row.block,
        pids: pids(&row.pids),
    })
}

/// 拉取一次性 stats 快照（--no-stream），返回所有容器的采样。
pub async fn snapshot(client: &mut SshClient) -> AppResult<Vec<StatsSample>> {
    let res = client
        .exec("docker stats --no-stream --format '{{json .}}'")
        .await?;
    let mut out = Vec::new();
    for line in res.stdout.lines() {
        if let Some(s) = parse_line(line) {
            out.push(s);
        }
    }
    Ok(out)
}

/// 构造流式 stats 命令（每 N 秒一行 JSON）。
///
/// docker stats 默认持续流式输出（约 1 次/秒），`--interval` 在部分旧版/精简发行版
/// 不被支持（会报 Usage 错误），因此不传 interval，靠默认刷新即可。
/// docker 的 Go template 要求双大括号 {{ }}，用 raw 字符串避免转义混乱。
pub fn build_stream_cmd(_interval_secs: u64) -> String {
    format!("docker stats --format '{{{{json .}}}}'")
}
