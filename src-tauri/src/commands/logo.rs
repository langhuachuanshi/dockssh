//! 容器镜像 logo 缓存 commands。
//!
//! 缓存目录：AppData / dockssh / logos / <image>.svg
//! 读取策略：本地缓存优先 → 命中则直接返回；
//!          未命中则前端按需调用 fetch_logo 从 iconify 下载并写入缓存。
//!
//! 设计为三步（前端编排）：
//!   1. get_cached_logo(slug)  —— 读本地缓存，未命中返回 null
//!   2. fetch_logo(slug, prefix) —— 从 iconify 下载，成功后写入缓存并返回
//!   3. 前端两者都失败 → 渲染通用容器图标（纯前端 SVG）

use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};

/// logos 缓存目录绝对路径。
fn logos_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(format!("获取 app_data_dir 失败: {e}")))?
        .join("logos");
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// slug 规范化：仅允许 [a-z0-9._-]，防止路径穿越。
fn safe_slug(slug: &str) -> AppResult<String> {
    let s = slug.trim().to_lowercase();
    if s.is_empty() {
        return Err(AppError::Other("slug 不能为空".into()));
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        return Err(AppError::Other(format!("非法 slug: {slug}")));
    }
    Ok(s)
}

/// 读取本地缓存的 logo SVG。未命中返回 None。
#[tauri::command]
pub async fn get_cached_logo(app: AppHandle, slug: String) -> AppResult<Option<String>> {
    let slug = safe_slug(&slug)?;
    let path = logos_dir(&app)?.join(format!("{slug}.svg"));
    if path.exists() {
        let content = fs::read_to_string(path)?;
        Ok(Some(content))
    } else {
        Ok(None)
    }
}

/// 清除某 logo 缓存。
#[tauri::command]
pub async fn delete_cached_logo(app: AppHandle, slug: String) -> AppResult<()> {
    let slug = safe_slug(&slug)?;
    let path = logos_dir(&app)?.join(format!("{slug}.svg"));
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// 清空所有 logo 缓存。
#[tauri::command]
pub async fn clear_logo_cache(app: AppHandle) -> AppResult<()> {
    let dir = logos_dir(&app)?;
    if dir.exists() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

/// 从 iconify 下载 logo 并写入缓存。
///
/// 按图标集前缀依次尝试，首个成功即返回。
/// 常用前缀：simple-icons / logos / devicon。
/// 默认尝试 ["simple-icons", "logos", "devicon"]。
#[tauri::command]
pub async fn fetch_logo(
    app: AppHandle,
    slug: String,
    prefixes: Option<Vec<String>>,
) -> AppResult<String> {
    let slug = safe_slug(&slug)?;
    // 先查缓存，避免重复下载
    let path = logos_dir(&app)?.join(format!("{slug}.svg"));
    if path.exists() {
        return Ok(fs::read_to_string(&path)?);
    }

    let prefixes = prefixes.unwrap_or_else(|| {
        vec![
            "simple-icons".to_string(),
            "logos".to_string(),
            "devicon".to_string(),
        ]
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("dockssh")
        .build()
        .map_err(|e| AppError::Other(format!("构建 HTTP 客户端失败: {e}")))?;

    let mut last_err = String::new();
    for prefix in &prefixes {
        let prefix_safe = safe_slug(prefix)?;
        let url = format!("https://api.iconify.design/{prefix_safe}/{slug}.svg");
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let text = resp
                    .text()
                    .await
                    .map_err(|e| AppError::Other(format!("读取响应失败: {e}")))?;
                // 简单校验：必须是 SVG
                let trimmed = text.trim_start();
                if !trimmed.starts_with("<svg") && !trimmed.starts_with("<?xml") {
                    last_err = format!("{prefix_safe}: 响应非 SVG");
                    continue;
                }
                // 写入缓存
                if let Err(e) = fs::write(&path, &text) {
                    return Err(AppError::Io(e));
                }
                return Ok(text);
            }
            Ok(resp) => {
                last_err = format!("{prefix_safe}: HTTP {}", resp.status());
            }
            Err(e) => {
                last_err = format!("{prefix_safe}: {e}");
            }
        }
    }

    Err(AppError::Other(format!("iconify 下载失败: {last_err}")))
}
