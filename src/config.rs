use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    // 使用 HashMap 存储: alias -> user@host
    pub connections: HashMap<String, String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Config::default());
        }
        let content = fs::read_to_string(path).context("Failed to read config file")?;
        let config: Config = serde_json::from_str(&content).context("Failed to parse config file")?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).context("Failed to create config directory")?;
        let content = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(path, content).context("Failed to write config file")?;
        Ok(())
    }
}

// 辅助函数，获取配置文件路径
pub fn config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    Ok(home_dir.join(".rss_ssh/config.json"))
}
