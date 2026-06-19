//! 文件管理 commands：基于 SFTP 实现远程文件浏览/读写/上传/下载。
//!
//! 设计要点：
//! - SFTP 会话由 `SshClient::sftp()` 懒加载并缓存，多次 command 复用同一会话。
//! - 所有路径在前端传绝对路径；这里对 path 做 canonicalize 规范化后再用，
//!   一方面拿到真实绝对路径（如 "." 会被解析成主目录），另一方面验证可访问。
//! - 文本预览限制 1MB，避免把超大文件整块读进内存。

use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use russh_sftp::client::fs::DirEntry;
use russh_sftp::protocol::FileAttributes;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::{DirListing, FileEntry};
use crate::state::SharedState;

/// 文本预览的大小上限（1MB）。
const TEXT_PREVIEW_MAX: usize = 1024 * 1024;

/// 当前 SSH 登录用户的主目录（绝对路径）。前端首次进入「文件」时调用。
#[tauri::command]
pub async fn file_home(state: State<'_, SharedState>, host_id: String) -> AppResult<String> {
    let arc = state.pool.get(&host_id).await?;
    let client = arc.lock().await;
    let sftp = client.sftp().await?;
    sftp.canonicalize(".")
        .await
        .map_err(|e| AppError::Ssh(format!("解析主目录失败: {e}")))
}

/// 列出某目录下的条目（目录优先，同类按名称排序）。
#[tauri::command]
pub async fn list_dir(
    state: State<'_, SharedState>,
    host_id: String,
    path: String,
) -> AppResult<DirListing> {
    let arc = state.pool.get(&host_id).await?;
    let client = arc.lock().await;
    let sftp = client.sftp().await?;

    // 规范化路径（解析 . / .. / 符号链接），同时作为可访问性校验
    let real = sftp
        .canonicalize(&path)
        .await
        .map_err(|e| AppError::Ssh(format!("无法访问目录「{path}」: {e}")))?;

    let mut read_dir = sftp
        .read_dir(real.clone())
        .await
        .map_err(|e| AppError::Ssh(format!("读取目录失败: {e}")))?;

    let mut entries = Vec::new();
    while let Some(entry) = read_dir.next() {
        entries.push(entry_to_file_entry(entry));
    }

    // 排序：目录在前，同类按名称（不区分大小写）
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(DirListing { path: real, entries })
}

/// 读取小文本文件内容用于预览（>1MB 拒绝）。
#[tauri::command]
pub async fn file_read_text(
    state: State<'_, SharedState>,
    host_id: String,
    path: String,
) -> AppResult<String> {
    let arc = state.pool.get(&host_id).await?;
    let client = arc.lock().await;
    let sftp = client.sftp().await?;

    let meta = sftp
        .metadata(&path)
        .await
        .map_err(|e| AppError::Ssh(format!("获取文件信息失败: {e}")))?;
    if meta.is_dir() {
        return Err(AppError::Other("目标是目录，无法预览".into()));
    }
    let size = meta.len() as usize;
    if size > TEXT_PREVIEW_MAX {
        return Err(AppError::Other(format!(
            "文件过大（{} 字节），预览上限 1MB",
            meta.len()
        )));
    }
    let bytes = sftp
        .read(&path)
        .await
        .map_err(|e| AppError::Ssh(format!("读取文件失败: {e}")))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// 新建目录。
#[tauri::command]
pub async fn file_mkdir(
    state: State<'_, SharedState>,
    host_id: String,
    path: String,
) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let client = arc.lock().await;
    let sftp = client.sftp().await?;
    sftp.create_dir(&path)
        .await
        .map_err(|e| AppError::Ssh(format!("创建目录失败: {e}")))
}

/// 删除文件或空目录（is_dir 决定调用哪个 SFTP 原语）。
#[tauri::command]
pub async fn file_remove(
    state: State<'_, SharedState>,
    host_id: String,
    path: String,
    is_dir: bool,
) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let client = arc.lock().await;
    let sftp = client.sftp().await?;
    let r = if is_dir {
        sftp.remove_dir(&path).await
    } else {
        sftp.remove_file(&path).await
    };
    r.map_err(|e| AppError::Ssh(format!("删除失败: {e}")))
}

/// 重命名/移动文件或目录。
#[tauri::command]
pub async fn file_rename(
    state: State<'_, SharedState>,
    host_id: String,
    from: String,
    to: String,
) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let client = arc.lock().await;
    let sftp = client.sftp().await?;
    sftp.rename(&from, &to)
        .await
        .map_err(|e| AppError::Ssh(format!("重命名失败: {e}")))
}

/// 下载远程文件到本地。local 为本地绝对路径（由前端 dialog 给出）。
#[tauri::command]
pub async fn file_download(
    state: State<'_, SharedState>,
    host_id: String,
    remote: String,
    local: String,
) -> AppResult<()> {
    let arc = state.pool.get(&host_id).await?;
    let client = arc.lock().await;
    let sftp = client.sftp().await?;
    let bytes = sftp
        .read(&remote)
        .await
        .map_err(|e| AppError::Ssh(format!("下载读取失败: {e}")))?;
    let local_path = PathBuf::from(&local);
    if let Some(parent) = local_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&local_path, &bytes).await?;
    Ok(())
}

/// 上传本地文件到远程目录。local 为本地文件绝对路径，remote_dir 为远程目录。
/// 远程目标路径 = remote_dir / 文件名。
#[tauri::command]
pub async fn file_upload(
    state: State<'_, SharedState>,
    host_id: String,
    local: String,
    remote_dir: String,
) -> AppResult<()> {
    let local_path = PathBuf::from(&local);
    let file_name = local_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::Other("无法解析本地文件名".into()))?
        .to_string();
    let bytes = tokio::fs::read(&local_path).await?;

    let remote_path = if remote_dir.ends_with('/') {
        format!("{remote_dir}{file_name}")
    } else {
        format!("{remote_dir}/{file_name}")
    };

    let arc = state.pool.get(&host_id).await?;
    let client = arc.lock().await;
    let sftp = client.sftp().await?;
    sftp.write(&remote_path, &bytes)
        .await
        .map_err(|e| AppError::Ssh(format!("上传写入失败: {e}")))?;
    Ok(())
}

/// 把 SFTP DirEntry 转成前端用的 FileEntry。
fn entry_to_file_entry(entry: DirEntry) -> FileEntry {
    let name = entry.file_name();
    let attrs: FileAttributes = entry.metadata();

    let modified = attrs
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    FileEntry {
        name,
        is_dir: attrs.is_dir(),
        is_symlink: attrs.is_symlink(),
        size: attrs.len(),
        modified,
        permissions: format_permissions(&attrs),
    }
}

/// 把 SFTP 权限位格式化成 "rwxr-xr-x" 形式；拿不到权限信息时返回 None。
fn format_permissions(attrs: &FileAttributes) -> Option<String> {
    let bits = attrs.permissions?;
    let mut out = String::with_capacity(9);
    let triplets = [(bits >> 6) & 0o7, (bits >> 3) & 0o7, bits & 0o7];
    for t in triplets {
        out.push(if t & 4 != 0 { 'r' } else { '-' });
        out.push(if t & 2 != 0 { 'w' } else { '-' });
        out.push(if t & 1 != 0 { 'x' } else { '-' });
    }
    Some(out)
}
