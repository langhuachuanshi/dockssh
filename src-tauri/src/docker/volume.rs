//! 存储卷相关命令：列表 / 详情 / 删除。
//!
//! 列表用 `docker volume ls --format '{{json .}}'`，再对每个卷跑一次
//! `docker volume inspect` 取 Mountpoint 与 CreatedAt。

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::{Volume, VolumeInspect};
use crate::ssh::client::SshClient;

#[derive(Debug, Deserialize)]
struct VolumeRow {
    #[serde(rename = "Driver", default)]
    driver: String,
    #[serde(rename = "Name", default)]
    name: String,
}

/// 列出所有 docker 存储卷。
///
/// 对每个卷追加一次 `docker volume inspect`，取 Mountpoint / CreatedAt。
/// 单卷 inspect 失败时仅留空字段，不阻断整批列表。
pub async fn list(client: &mut SshClient) -> AppResult<Vec<Volume>> {
    let res = client
        .exec("docker volume ls --format '{{json .}}'")
        .await?;

    let mut out = Vec::new();
    for line in res.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<VolumeRow>(line) else {
            continue;
        };
        let (mountpoint, created) = inspect_fields(client, &row.name).await.unwrap_or_default();
        out.push(Volume {
            driver: row.driver,
            name: row.name,
            mountpoint,
            created,
        });
    }
    Ok(out)
}

/// 取卷的 Mountpoint / CreatedAt 原始字符串。失败返回空串对。
async fn inspect_fields(
    client: &mut SshClient,
    name: &str,
) -> AppResult<(String, String)> {
    // 自定义分隔符，避免路径里出现空白被截断
    let cmd = format!(
        "docker volume inspect --format '{{{{.Mountpoint}}}}\x1f{{{{.CreatedAt}}}}' {name}"
    );
    let res = client.exec(&cmd).await?;
    if res.exit_code != 0 {
        return Err(AppError::Docker(format!(
            "volume inspect 失败: {}",
            res.stderr.trim()
        )));
    }
    let out = res.stdout.trim();
    let mut parts = out.splitn(2, '\x1f');
    let mountpoint = parts.next().map(|s| s.trim().to_string()).unwrap_or_default();
    let created = parts.next().map(|s| s.trim().to_string()).unwrap_or_default();
    Ok((mountpoint, created))
}

/// 查询卷详情（用于「打开目录」跳转），取 Mountpoint。
pub async fn inspect(client: &mut SshClient, name: &str) -> AppResult<VolumeInspect> {
    let cmd = format!("docker volume inspect --format '{{{{.Mountpoint}}}}' {name}");
    let res = client.exec(&cmd).await?;
    if res.exit_code != 0 {
        return Err(AppError::Docker(format!(
            "volume inspect 失败: {}",
            res.stderr.trim()
        )));
    }
    Ok(VolumeInspect {
        mountpoint: res.stdout.trim().to_string(),
    })
}

/// 删除卷。force=true 时强制（即使有容器引用）。
pub async fn remove(client: &mut SshClient, name: &str, force: bool) -> AppResult<()> {
    let flag = if force { " -f" } else { "" };
    let res = client.exec(&format!("docker volume rm{flag} {name}")).await?;
    if res.exit_code == 0 {
        Ok(())
    } else {
        Err(AppError::Docker(format!(
            "退出码 {}: {}",
            res.exit_code,
            res.stderr.trim()
        )))
    }
}
