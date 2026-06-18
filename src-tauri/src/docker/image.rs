//! 镜像相关命令：列表 / 删除。
//!
//! 列表用 `docker images --format '{{json .}}'`。
//! 拉取（pull）是流式的，放在 commands 层处理进度事件，这里不提供。

use serde::Deserialize;

use crate::error::AppResult;
use crate::models::Image;
use crate::ssh::client::SshClient;

#[derive(Debug, Deserialize)]
struct ImageRow {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Repository", default)]
    repository: String,
    #[serde(rename = "Tag", default)]
    tag: String,
    #[serde(rename = "Size", default)]
    size: String,
    #[serde(rename = "CreatedSince", default)]
    created: String,
}

/// 列出所有镜像。
pub async fn list(client: &mut SshClient) -> AppResult<Vec<Image>> {
    let res = client
        .exec("docker images --format '{{json .}}'")
        .await?;

    let mut out = Vec::new();
    for line in res.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(row) = serde_json::from_str::<ImageRow>(line) {
            out.push(Image {
                id: row.id,
                repository: row.repository,
                tag: row.tag,
                size: row.size,
                created: row.created,
            });
        }
    }
    Ok(out)
}

/// 删除镜像。force=true 时强制（即使有容器引用）。
pub async fn remove(client: &mut SshClient, id: &str, force: bool) -> AppResult<()> {
    let flag = if force { " -f" } else { "" };
    let res = client.exec(&format!("docker rmi{flag} {id}")).await?;
    if res.exit_code == 0 {
        Ok(())
    } else {
        Err(crate::error::AppError::Docker(format!(
            "退出码 {}: {}",
            res.exit_code,
            res.stderr.trim()
        )))
    }
}
