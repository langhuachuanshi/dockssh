//! docker compose（v2 插件）相关命令。
//!
//! 支持两种作用域：
//! - 不传 project_dir：列出系统检测到的所有 compose 项目（靠 labels 识别容器）
//! - 传 project_dir：在指定目录执行 up/down/ps/logs
//!
//! 第一版聚焦"列出 compose 项目"和"对单项目执行操作"。

use crate::error::AppResult;
use crate::ssh::client::SshClient;

/// compose 项目摘要。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComposeProject {
    pub name: String,
    pub containers: usize,
}

/// 通过扫描容器 labels 中的 `com.docker.compose.project` 聚合出 compose 项目列表。
pub async fn list_projects(client: &mut SshClient) -> AppResult<Vec<ComposeProject>> {
    let res = client
        .exec("docker ps -a --format '{{.Label \"com.docker.compose.project\"}}'")
        .await?;

    let mut counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for line in res.stdout.lines() {
        let name = line.trim();
        if name.is_empty() {
            continue;
        }
        *counts.entry(name.to_string()).or_insert(0) += 1;
    }
    let mut out: Vec<_> = counts
        .into_iter()
        .map(|(name, containers)| ComposeProject { name, containers })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// 在指定 compose 项目目录执行命令。
/// `what` 为 "up" / "down" / "ps" / "logs" 等。
pub async fn run(client: &mut SshClient, project_dir: &str, what: &str) -> AppResult<String> {
    let cmd = format!("docker compose -f {project_dir}/docker-compose.yml {what} 2>&1");
    let res = client.exec(&cmd).await?;
    Ok(format!("{}\n{}", res.stdout, res.stderr))
}
