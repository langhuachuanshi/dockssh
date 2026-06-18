//! 容器相关命令：列表 / 启停 / 重启 / 删除 / 重命名。
//!
//! 列表用 `docker ps --format '{{json .}}'`，每行一个 JSON 对象，
//! 避免 parsing 对齐表格的脆弱性。

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::Container;
use crate::ssh::client::SshClient;

/// docker ps 单行 JSON 的原始结构。
#[derive(Debug, Deserialize)]
struct PsRow {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Names", default)]
    names: String,
    #[serde(rename = "Image", default)]
    image: String,
    #[serde(rename = "Command", default)]
    command: String,
    #[serde(rename = "State", default)]
    state: String,
    #[serde(rename = "Status", default)]
    status: String,
    #[serde(rename = "Ports", default)]
    ports: String,
    #[serde(rename = "Labels", default)]
    labels: String,
}

/// 列出所有容器（含已停止）。compose_project 从 labels 里提取。
pub async fn list(client: &mut SshClient) -> AppResult<Vec<Container>> {
    let res = client
        .exec("docker ps -a --format '{{json .}}'")
        .await?;

    let mut out = Vec::new();
    for line in res.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: PsRow = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let compose_project = row
            .labels
            .split(',')
            .find_map(|kv| {
                let kv = kv.trim();
                kv.strip_prefix("com.docker.compose.project=")
                    .map(|v| v.to_string())
            });
        out.push(Container {
            id: row.id,
            name: row.names,
            image: row.image,
            command: row.command,
            state: row.state,
            status: row.status,
            ports: crate::docker::parse::parse_ports(&row.ports),
            compose_project,
        });
    }
    Ok(out)
}

/// 启动容器。
pub async fn start(client: &mut SshClient, id_or_name: &str) -> AppResult<()> {
    let cmd = format!("docker start {id_or_name}");
    let res = client.exec(&cmd).await?;
    check_exit(&res.stderr, res.exit_code)
}

/// 停止容器。
pub async fn stop(client: &mut SshClient, id_or_name: &str) -> AppResult<()> {
    let res = client.exec(&format!("docker stop {id_or_name}")).await?;
    check_exit(&res.stderr, res.exit_code)
}

/// 重启容器。
pub async fn restart(client: &mut SshClient, id_or_name: &str) -> AppResult<()> {
    let res = client
        .exec(&format!("docker restart {id_or_name}"))
        .await?;
    check_exit(&res.stderr, res.exit_code)
}

/// 删除容器（默认强制 -f，C 端工具行为；如需温和删除前端可加确认）。
pub async fn remove(client: &mut SshClient, id_or_name: &str, force: bool) -> AppResult<()> {
    let flag = if force { " -f" } else { "" };
    let res = client
        .exec(&format!("docker rm{flag} {id_or_name}"))
        .await?;
    check_exit(&res.stderr, res.exit_code)
}

fn check_exit(stderr: &str, code: i32) -> AppResult<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(AppError::Docker(format!(
            "退出码 {code}: {}",
            stderr.trim()
        )))
    }
}
