//! 应用级配置文件：`~/.config/rss-reader/config.json`。
//! 目前用于记录数据目录覆盖（data_dir），供桌面端/TUI 在启动时读取。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// 自定义数据目录（内含 rss.db）。`None` 表示使用默认目录。
    pub data_dir: Option<PathBuf>,
}

pub fn config_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rss-reader")
        .join("config.json")
}

pub fn load() -> AppConfig {
    let path = config_file_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .unwrap_or_default()
}

pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_file_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// 读取自定义数据目录（若无覆盖则为 None）。
pub fn load_data_dir() -> Option<PathBuf> {
    load().data_dir
}

/// 设置/清除自定义数据目录覆盖。
pub fn save_data_dir(dir: Option<PathBuf>) -> Result<(), String> {
    let mut config = load();
    config.data_dir = dir;
    save(&config)
}
