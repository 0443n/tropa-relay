use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyEntry {
    pub name: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub username: String,
    pub password: String,
    pub local_port: u16,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub proxies: Vec<ProxyEntry>,
    #[serde(default)]
    pub autostart: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            proxies: vec![],
            autostart: false,
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("could not determine config directory")
        .join("tropa-relay")
        .join("config.toml")
}

impl AppConfig {
    pub fn load() -> Self {
        let path = config_path();
        match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("warning: failed to parse config: {e}, using defaults");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self).expect("failed to serialize config");
        fs::write(&path, contents)
    }
}
