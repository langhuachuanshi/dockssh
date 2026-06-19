//! docker compose（v2 插件）相关命令。
//!
//! 支持两种作用域：
//! - 不传 project_dir：列出系统检测到的所有 compose 项目（靠 labels 识别容器）
//! - 传 project_dir：在指定目录执行 up/down/ps/logs
//!
//! 本模块聚焦：
//! 1. 列出 compose 项目（含运行/创建时间/compose 文件路径，全部从容器 labels 推导）
//! 2. 读取 / 回写 compose 文件（编辑器入口）
//! 3. `docker compose -p <name> down`（删除/停止一组编排）
//!
//! 注意：路径/文件内容经 SSH 文本传输，避免在容器主机之外的 shell 上解析。

use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::ComposeProject;
use crate::ssh::client::SshClient;

/// heredoc 结束标记（带前缀，内容中出现同名行的概率极低）。
const EOF_MARKER: &str = "DOCKSSH_EOF";

/// 把 docker 的 CreatedAt（形如 `2024-01-02 03:04:05 +0000 UTC`）
/// 格式化成 `2024-01-02 03:04:05`。解析失败时原样返回。
fn format_created(raw: &str) -> String {
    let s = raw.trim();
    // 取前 19 位 "YYYY-MM-DD HH:MM:SS"
    if s.len() >= 19 {
        let head = &s[..19];
        if let Ok(_) = NaiveDateTime::parse_from_str(head, "%Y-%m-%d %H:%M:%S") {
            return head.to_string();
        }
    }
    raw.to_string()
}

/// `docker ps -a --format '{{json .}}'` 单行结构（只取 compose 感兴趣字段）。
#[derive(Debug, Deserialize)]
struct PsRow {
    #[serde(rename = "State", default)]
    state: String,
    #[serde(rename = "CreatedAt", default)]
    created_at: String,
    #[serde(rename = "Labels", default)]
    labels: String,
}

/// 把 "k1=v1,k2=v2,..." 形式的 labels 字符串解析成 HashMap。
fn parse_labels(raw: &str) -> HashMap<String, String> {
    raw.split(',')
        .filter_map(|kv| {
            let kv = kv.trim();
            let (k, v) = kv.split_once('=')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

/// 每个项目的聚合中间结构。
#[derive(Default)]
struct ProjectAgg {
    containers: usize,
    running: usize,
    /// 最早创建时间（字符串比较即可，docker 输出为固定格式时间串）。
    earliest: Option<String>,
    /// 取组内第一条 config_files。
    config_files: Option<String>,
}

/// 通过扫描容器 labels 中的 `com.docker.compose.project` 聚合出 compose 项目列表。
///
/// 同时从每个容器推导出：
/// - 运行中数量（State == "running"）
/// - 创建时间（取组内 CreatedAt 最小值）
/// - compose 文件路径（label `com.docker.compose.project.config_files`）
pub async fn list_projects(client: &mut SshClient) -> AppResult<Vec<ComposeProject>> {
    let res = client
        .exec("docker ps -a --format '{{json .}}'")
        .await?;

    let mut agg: HashMap<String, ProjectAgg> = HashMap::new();
    for line in res.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: PsRow = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let labels = parse_labels(&row.labels);
        let Some(name) = labels.get("com.docker.compose.project") else {
            // 非 compose 启动的容器，跳过
            continue;
        };
        let entry = agg.entry(name.clone()).or_default();
        entry.containers += 1;
        if row.state == "running" {
            entry.running += 1;
        }
        // 创建时间：取最小（最早）。空串不参与。
        if !row.created_at.is_empty() {
            let earliest = entry
                .earliest
                .as_deref()
                .unwrap_or(&row.created_at)
                .min(&row.created_at)
                .to_string();
            entry.earliest = Some(earliest);
        }
        // compose 文件路径：取组内第一条非空值
        if entry.config_files.is_none() {
            if let Some(cf) = labels.get("com.docker.compose.project.config_files") {
                if !cf.is_empty() {
                    entry.config_files = Some(cf.clone());
                }
            }
        }
    }

    let mut out: Vec<_> = agg
        .into_iter()
        .map(|(name, a)| ComposeProject {
            name,
            containers: a.containers,
            running: a.running,
            created: format_created(&a.earliest.unwrap_or_default()),
            config_files: a.config_files,
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// 读取 compose 文件内容。
///
/// 路径通常来自 label `com.docker.compose.project.config_files`，为容器主机上的绝对路径。
pub async fn read_file(client: &mut SshClient, path: &str) -> AppResult<String> {
    let res = client.exec(&format!("cat {path:?}")).await?;
    if res.exit_code == 0 {
        Ok(res.stdout)
    } else {
        Err(AppError::Docker(format!(
            "退出码 {}: {}",
            res.exit_code,
            res.stderr.trim()
        )))
    }
}

/// 把内容覆盖写回 compose 文件。
///
/// 用 quoted heredoc（`<<'EOF'`，单引号）避免 `$`/反引号被 shell 解释，
/// 内容本身原样写入。路径用 `{path:?}` 做基础转义。
pub async fn write_file(client: &mut SshClient, path: &str, content: &str) -> AppResult<()> {
    // 内容里若出现结束标记行，写入会提前截断。极罕见，做一次校验并拒绝。
    let end_marker = EOF_MARKER;
    for line in content.lines() {
        if line.trim() == end_marker {
            return Err(AppError::Other(format!(
                "文件内容包含 heredoc 结束标记 {end_marker:?}，已拒绝写入以避免截断"
            )));
        }
    }

    let cmd = format!(
        "cat > {path:?} <<'{end_marker}'\n{content}\n{end_marker}\n",
    );
    let res = client.exec(&cmd).await?;
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

/// 停止并移除一个 compose 项目的容器/网络（默认不删卷、不删镜像）。
///
/// 用 `docker compose -p <name> down`，按项目名操作，不依赖文件目录。
/// 回传合并的 stdout+stderr，便于前端展示细节。
pub async fn down(client: &mut SshClient, project_name: &str) -> AppResult<String> {
    run_compose(client, project_name, "down").await
}

/// 构建项目镜像：`docker compose -p <name> build`。
pub async fn build(client: &mut SshClient, project_name: &str) -> AppResult<String> {
    run_compose(client, project_name, "build").await
}

/// 后台拉起项目：`docker compose -p <name> up -d`。
pub async fn up(client: &mut SshClient, project_name: &str) -> AppResult<String> {
    run_compose(client, project_name, "up -d").await
}

/// 仅停止项目容器（不删除容器/网络）：`docker compose -p <name> stop`。
pub async fn stop(client: &mut SshClient, project_name: &str) -> AppResult<String> {
    run_compose(client, project_name, "stop").await
}

/// 重启项目容器：`docker compose -p <name> restart`。
pub async fn restart(client: &mut SshClient, project_name: &str) -> AppResult<String> {
    run_compose(client, project_name, "restart").await
}

/// 统一的 `docker compose -p <name> <args> 2>&1` 执行器。
///
/// `2>&1` 合并 stderr 到 stdout（compose 常把进度打到 stderr），按退出码判断成败。
async fn run_compose(
    client: &mut SshClient,
    project_name: &str,
    args: &str,
) -> AppResult<String> {
    let cmd = format!("docker compose -p {project_name:?} {args} 2>&1");
    let res = client.exec(&cmd).await?;
    if res.exit_code == 0 {
        Ok(res.stdout)
    } else {
        Err(AppError::Docker(format!(
            "退出码 {}: {}",
            res.exit_code,
            res.stderr.trim()
        )))
    }
}
