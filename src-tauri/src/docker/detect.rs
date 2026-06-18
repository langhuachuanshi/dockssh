//! 目标机环境探测：识别 OS、是否 WSL2、是否 Windows 原生容器、Docker 版本、compose 可用性。
//!
//! 探测策略（一次 SSH 连续执行多条轻量命令，减少往返）：
//! - uname / 环境变量判断 OS
//! - docker version 取 client/server 版本
//! - docker compose version 判断 v2 插件
//! - 检测 WSL2：/proc/sys/kernel/osrelease 包含 microsoft

use crate::error::AppResult;
use crate::models::HostProbe;
use crate::ssh::client::SshClient;

/// 一条复合探测脚本，输出多行 key=value，再由 Rust 端解析。
/// 用 `;` 分隔保证即便某条失败也继续，最后用稳定标记行收尾。
const PROBE_SCRIPT: &str = r#"
echo "DSSH_HOSTNAME=$(hostname 2>/dev/null)";
echo "DSSH_OS=$([[ -n "$WINDIR" ]] && echo windows || uname -s 2>/dev/null || echo unknown)";
echo "DSSH_OSREL=$(uname -r 2>/dev/null || echo '')";
echo "DSSH_DOCKER_VER=$(docker version --format '{{.Server.Version}}' 2>/dev/null || echo '')";
echo "DSSH_DOCKER_CLIENT=$(docker version --format '{{.Client.Version}}' 2>/dev/null || echo '')";
echo "DSSH_COMPOSE=$(docker compose version --short 2>/dev/null || echo '')";
echo "DSSH_PROBE_END";
"#;

/// 执行探测并返回结构化结果。
pub async fn probe(client: &mut SshClient) -> AppResult<HostProbe> {
    let result = client.exec(PROBE_SCRIPT).await?;

    // 即使命令出错也尝试解析，因为脚本内部已吞掉错误。
    let mut map = std::collections::HashMap::new();
    for line in result.stdout.lines() {
        if let Some((k, v)) = line.trim().split_once('=') {
            if k.starts_with("DSSH_") {
                map.insert(k.to_string(), v.trim_matches('"').to_string());
            }
        }
    }

    let os_raw = map.get("DSSH_OS").cloned().unwrap_or_default();
    let osrel = map.get("DSSH_OSREL").cloned().unwrap_or_default();
    let os = if os_raw.eq_ignore_ascii_case("windows") {
        "windows".to_string()
    } else {
        os_raw.to_lowercase()
    };

    // WSL2：内核版本字符串含 microsoft-standard 或 microsoft
    let is_wsl2 = osrel
        .to_lowercase()
        .contains("microsoft");

    // Windows 原生容器：OS=windows 且非 WSL2（WSL2 下 uname 仍是 linux）
    // 注：纯 Windows 服务器 SSH 进去通常默认 shell 是 cmd/PowerShell，
    // uname 不存在，os_raw 可能是 windows。此时再依据 docker version 的 OS 字段。
    let is_windows_native = os == "windows" && !is_wsl2;

    Ok(HostProbe {
        os,
        is_wsl2,
        is_windows_native,
        docker_version: map.get("DSSH_DOCKER_VER").cloned().unwrap_or_default(),
        docker_client_version: map
            .get("DSSH_DOCKER_CLIENT")
            .cloned()
            .unwrap_or_default(),
        has_compose: !map
            .get("DSSH_COMPOSE")
            .map(String::is_empty)
            .unwrap_or(true),
        hostname: map.get("DSSH_HOSTNAME").cloned().unwrap_or_default(),
    })
}
