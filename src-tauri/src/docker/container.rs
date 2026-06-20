//! 容器相关命令：列表 / 启停 / 重启 / 删除 / 详情。
//!
//! 列表用 `docker ps --format '{{json .}}'`，每行一个 JSON 对象，
//! 避免 parsing 对齐表格的脆弱性。
//! 详情用 `docker inspect` 一次性拿完整 JSON，本地解析出前端所需字段。

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::{Container, ContainerInspect, ContainerMount, ContainerPortBinding};
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
    #[serde(rename = "CreatedAt", default)]
    created_at: String,
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
        created: row.created_at,
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

// ===== inspect 完整解析 =====
// docker inspect 返回的 JSON 体积大、字段多，这里只挑前端需要的子集。
// 所有字段用 #[serde(default)] 容错，避免新版 docker 增删字段导致解析失败。

/// docker inspect 单个挂载的原始结构。
#[derive(Debug, Deserialize)]
struct InspectMount {
    #[serde(rename = "Type", default)]
    typ: String,
    #[serde(rename = "Source", default)]
    source: String,
    #[serde(rename = "Destination", default)]
    destination: String,
}

#[derive(Debug, Deserialize)]
struct InspectRaw {
    #[serde(rename = "Id", default)]
    id: String,
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "Created", default)]
    created: String,
    #[serde(rename = "Image", default)]
    image: String,
    #[serde(default)]
    state: InspectState,
    #[serde(default)]
    config: InspectConfig,
    #[serde(default, rename = "NetworkSettings")]
    network: InspectNetwork,
    #[serde(default)]
    host_config: InspectHostConfig,
    #[serde(default)]
    mounts: Vec<InspectMount>,
}

#[derive(Debug, Default, Deserialize)]
struct InspectState {
    #[serde(default)]
    status: String,
    #[serde(default)]
    running: bool,
    #[serde(default)]
    exit_code: i64,
    #[serde(default)]
    pid: i64,
    #[serde(default)]
    started_at: String,
    #[serde(default)]
    finished_at: String,
}

#[derive(Debug, Default, Deserialize)]
struct InspectConfig {
    #[serde(default)]
    working_dir: String,
    #[serde(default)]
    entrypoint: Vec<String>,
    #[serde(default)]
    cmd: Vec<String>,
    #[serde(default)]
    env: Vec<String>,
    #[serde(default, rename = "ExposedPorts")]
    exposed_ports: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Default, Deserialize)]
struct InspectNetwork {
    #[serde(default, rename = "IPAddress")]
    ip_address: String,
    #[serde(default, rename = "Gateway")]
    gateway: String,
    #[serde(default, rename = "MacAddress")]
    mac_address: String,
    /// 真正的 IP/Gateway/MAC 在每个子网络里，顶层这几个通常为空
    #[serde(default, rename = "Networks")]
    networks: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct InspectHostConfig {
    #[serde(default)]
    restart_policy: InspectRestartPolicy,
    #[serde(default, rename = "PortBindings")]
    port_bindings: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Default, Deserialize)]
struct InspectRestartPolicy {
    #[serde(default)]
    name: String,
    #[serde(default)]
    maximum_retry_count: i64,
}

/// 单个端口绑定的内层结构：[{ "HostIp": "0.0.0.0", "HostPort": "8080" }]
#[derive(Debug, Deserialize)]
struct InspectPortBindingInner {
    #[serde(default, rename = "HostIp")]
    host_ip: String,
    #[serde(default, rename = "HostPort")]
    host_port: String,
}

/// 查询容器详情。一次 `docker inspect` 拿完整 JSON，本地解析出前端所需字段。
pub async fn inspect(client: &mut SshClient, id_or_name: &str) -> AppResult<ContainerInspect> {
    let cmd = format!("docker inspect {id_or_name}");
    let res = client.exec(&cmd).await?;
    if res.exit_code != 0 {
        return Err(AppError::Docker(format!(
            "inspect 失败: {}",
            res.stderr.trim()
        )));
    }
    // docker inspect 返回数组
    let raws: Vec<InspectRaw> = serde_json::from_str(res.stdout.trim())
        .map_err(|e| AppError::Parse(format!("解析 inspect 失败: {e}")))?;
    let raw = raws
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Docker("inspect 返回空数组".into()))?;

    // 从 Networks 子节点取首个网络的 IP/Gateway/MAC（顶层通常为空）
    let mut ip = raw.network.ip_address.clone();
    let mut gw = raw.network.gateway.clone();
    let mut mac = raw.network.mac_address.clone();
    let networks: Vec<String> = raw.network.networks.keys().cloned().collect();
    if (ip.is_empty() || gw.is_empty() || mac.is_empty()) && !raw.network.networks.is_empty() {
        if let Some(first) = raw.network.networks.values().next() {
            if let Some(obj) = first.as_object() {
                if ip.is_empty() {
                    ip = obj
                        .get("IPAddress")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
                if gw.is_empty() {
                    gw = obj
                        .get("Gateway")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
                if mac.is_empty() {
                    mac = obj
                        .get("MacAddress")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
            }
        }
    }

    // 端口绑定：{ "80/tcp": [{HostIp, HostPort}] }
    let mut port_bindings = Vec::new();
    if let Some(pb) = raw.host_config.port_bindings {
        for (cport, arr) in pb {
            if let Some(list) = arr.as_array() {
                for item in list {
                    if let Ok(b) = serde_json::from_value::<InspectPortBindingInner>(item.clone()) {
                        port_bindings.push(ContainerPortBinding {
                            container_port: cport.clone(),
                            host_ip: b.host_ip,
                            host_port: b.host_port,
                        });
                    }
                }
            }
        }
    }

    // ExposedPorts 的 key 列表
    let exposed_ports: Vec<String> = raw
        .config
        .exposed_ports
        .as_ref()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    // 名称去掉前导 /
    let name = raw
        .name
        .strip_prefix('/')
        .map(|s| s.to_string())
        .unwrap_or(raw.name);

    // status：running 时用 running 标记，否则用 state.status
    let status = if raw.state.running {
        "running".to_string()
    } else {
        raw.state.status.clone()
    };

    Ok(ContainerInspect {
        id: raw.id,
        name,
        image: raw.image,
        created: raw.created,
        working_dir: raw.config.working_dir,
        entrypoint: raw.config.entrypoint,
        cmd: raw.config.cmd,
        env: raw.config.env,
        exposed_ports,
        state: raw.state.status,
        status,
        exit_code: raw.state.exit_code,
        started_at: raw.state.started_at,
        finished_at: raw.state.finished_at,
        pid: raw.state.pid,
        networks,
        ip_address: ip,
        gateway: gw,
        mac_address: mac,
        restart_policy: raw.host_config.restart_policy.name,
        restart_retries: raw.host_config.restart_policy.maximum_retry_count,
        port_bindings,
        mounts: raw
            .mounts
            .into_iter()
            .map(|m| ContainerMount {
                source: m.source,
                destination: m.destination,
                typ: m.typ,
            })
            .collect(),
    })
}
