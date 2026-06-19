//! 仓库（registry）相关命令：只读列出已登录的 registry。
//!
//! 通过读取远程机 `~/.docker/config.json` 的 `auths` 段得到已登录仓库列表。
//! 注意：只取 auths 的 key（仓库地址），不返回任何凭据信息。

use serde::Deserialize;

use crate::error::AppResult;
use crate::ssh::client::SshClient;

#[derive(Debug, Deserialize)]
struct DockerConfig {
    #[serde(default)]
    auths: std::collections::BTreeMap<String, serde_json::Value>,
}

/// 列出已登录的 registry 地址（来自 ~/.docker/config.json 的 auths 段）。
/// 返回排序后的仓库地址列表；config.json 不存在或无 auths 时返回空。
pub async fn list_logged_in(client: &mut SshClient) -> AppResult<Vec<String>> {
    // -s 文件存在才 cat，避免 cat 不存在文件把错误打到 stderr
    let res = client
        .exec("test -f ~/.docker/config.json && cat ~/.docker/config.json || true")
        .await?;

    let stdout = res.stdout.trim();
    if stdout.is_empty() {
        return Ok(Vec::new());
    }
    let cfg: DockerConfig = match serde_json::from_str(stdout) {
        Ok(c) => c,
        // 文件格式异常不算致命，按无数据返回
        Err(_) => return Ok(Vec::new()),
    };
    let mut out: Vec<String> = cfg.auths.keys().cloned().collect();
    out.sort();
    Ok(out)
}
