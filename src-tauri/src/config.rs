//! Persisted user config: manual zone override.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub manual_zone: Option<String>,
}

pub fn load(path: &Path) -> Config {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, config: &Config) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(config)?;
    std::fs::write(path, raw)
}

/// Default config location under the user's app-data dir (macOS).
pub fn default_config_path() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base)
        .join("Library/Application Support/com.khairul.waktu")
        .join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_manual_zone_through_disk() {
        let dir = std::env::temp_dir().join(format!("waktu-config-test-{}", std::process::id()));
        let path = dir.join("config.json");
        let config = Config {
            manual_zone: Some("WLY01".to_string()),
        };
        save(&path, &config).unwrap();
        let loaded = load(&path);
        assert_eq!(loaded.manual_zone.as_deref(), Some("WLY01"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_yields_default_config() {
        let path = std::env::temp_dir().join("waktu-config-does-not-exist.json");
        let loaded = load(&path);
        assert_eq!(loaded.manual_zone, None);
    }
}
