//! 凭据加密存储模块。
//!
//! 策略（C 端工具，安全与便捷兼顾）：
//! - 主机元信息（Host 结构，不含密码）→ 明文 JSON 文件，放 app data 目录
//! - 密码 / 私钥 passphrase → 系统密钥环（keyring crate），key = "dockssh:{host_id}"
//!
//! 这样即使 hosts.json 被拷走，没有系统密钥环也拿不到密码。
//! 私钥文件本身是用户已有的，我们只存路径，不复制内容。

use crate::error::{AppError, AppResult};

const SERVICE_NAME: &str = "DockSSH";

/// 凭据类型 → 密钥环 key 前缀。
const KIND_PASSWORD: &str = "pwd";
const KIND_PASSPHRASE: &str = "pass";

/// 通用：写入一条凭据到密钥环。
fn save_secret(host_id: &str, kind: &str, secret: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, &format!("{kind}:{host_id}"))
        .map_err(|e| AppError::Crypto(format!("打开密钥环失败: {e}")))?;
    entry
        .set_password(secret)
        .map_err(|e| AppError::Crypto(format!("写入凭据失败: {e}")))
}

/// 通用：读取一条凭据。
fn load_secret(host_id: &str, kind: &str) -> AppResult<String> {
    let entry = keyring::Entry::new(SERVICE_NAME, &format!("{kind}:{host_id}"))
        .map_err(|e| AppError::Crypto(format!("打开密钥环失败: {e}")))?;
    entry
        .get_password()
        .map_err(|e| AppError::Crypto(format!("读取凭据失败: {e}")))
}

/// 通用：删除一条凭据（不存在不算错）。
fn delete_secret(host_id: &str, kind: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, &format!("{kind}:{host_id}"))
        .map_err(|e| AppError::Crypto(format!("打开密钥环失败: {e}")))?;
    let _ = entry.delete_password();
    Ok(())
}

/// 把某主机的密码写入系统密钥环。
pub fn save_password(host_id: &str, password: &str) -> AppResult<()> {
    save_secret(host_id, KIND_PASSWORD, password)
}

/// 读取某主机的密码。
pub fn load_password(host_id: &str) -> AppResult<String> {
    load_secret(host_id, KIND_PASSWORD)
}

/// 把某主机私钥的口令(passphrase)写入系统密钥环。
pub fn save_passphrase(host_id: &str, passphrase: &str) -> AppResult<()> {
    save_secret(host_id, KIND_PASSPHRASE, passphrase)
}

/// 读取某主机私钥的口令。未设置时返回错误，调用方按需处理。
pub fn load_passphrase(host_id: &str) -> AppResult<String> {
    load_secret(host_id, KIND_PASSPHRASE)
}

/// 删除某主机的密码。
pub fn delete_password(host_id: &str) -> AppResult<()> {
    delete_secret(host_id, KIND_PASSWORD)
}

/// 删除某主机的私钥口令。
pub fn delete_passphrase(host_id: &str) -> AppResult<()> {
    delete_secret(host_id, KIND_PASSPHRASE)
}
