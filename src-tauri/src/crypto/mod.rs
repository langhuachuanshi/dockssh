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

/// 把某主机的密码写入系统密钥环。
pub fn save_password(host_id: &str, password: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, &format!("pwd:{host_id}"))
        .map_err(|e| AppError::Crypto(format!("打开密钥环失败: {e}")))?;
    entry
        .set_password(password)
        .map_err(|e| AppError::Crypto(format!("写入密码失败: {e}")))
}

/// 读取某主机的密码。
pub fn load_password(host_id: &str) -> AppResult<String> {
    let entry = keyring::Entry::new(SERVICE_NAME, &format!("pwd:{host_id}"))
        .map_err(|e| AppError::Crypto(format!("打开密钥环失败: {e}")))?;
    entry
        .get_password()
        .map_err(|e| AppError::Crypto(format!("读取密码失败: {e}")))
}

/// 删除某主机的密码。
pub fn delete_password(host_id: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, &format!("pwd:{host_id}"))
        .map_err(|e| AppError::Crypto(format!("打开密钥环失败: {e}")))?;
    let _ = entry.delete_password();
    Ok(())
}
