//! 网络相关命令：列表。
//!
//! 列表用 `docker network ls --format '{{json .}}'`。

use serde::Deserialize;

use crate::error::AppResult;
use crate::models::Network;
use crate::ssh::client::SshClient;

#[derive(Debug, Deserialize)]
struct NetworkRow {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "Driver", default)]
    driver: String,
    #[serde(rename = "Scope", default)]
    scope: String,
}

/// 列出所有 docker 网络。
pub async fn list(client: &mut SshClient) -> AppResult<Vec<Network>> {
    let res = client
        .exec("docker network ls --format '{{json .}}'")
        .await?;

    let mut out = Vec::new();
    for line in res.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(row) = serde_json::from_str::<NetworkRow>(line) {
            out.push(Network {
                id: row.id,
                name: row.name,
                driver: row.driver,
                scope: row.scope,
            });
        }
    }
    Ok(out)
}
