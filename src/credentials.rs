use keyring::Entry;
use anyhow::{Context, Result};

const SERVICE_NAME: &str = "rssh";

// 保存密码到系统的 keychain
pub fn set_password(alias: &str, password: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, alias)?;
    entry.set_password(password)
        .context(format!("Failed to save password for '{}'", alias))?;
    Ok(())
}

// 从系统的 keychain 获取密码
pub fn get_password(alias: &str) -> Result<String> {
    let entry = Entry::new(SERVICE_NAME, alias)?;
    entry.get_password()
        .context(format!("Failed to retrieve password for '{}'. Please run 'connect' command to set it first.", alias))
}

// 删除密码
pub fn delete_password(alias: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, alias)?;
    match entry.delete_password() {
        Ok(_) => Ok(()),
        // 如果密码不存在，也视为成功
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
