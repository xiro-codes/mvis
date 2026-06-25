use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::params::SimulationParams;

fn get_config_dir() -> PathBuf {
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg_config_home.is_empty() {
            return PathBuf::from(xdg_config_home).join("mvis");
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".config").join("mvis");
        }
    }

    // Fallback to current directory if neither is set
    PathBuf::from(".")
}

#[derive(Serialize, Deserialize, Clone, Resource)]
pub struct MpdConfig {
    pub host: String,
    pub fifo_path: String,
}

impl Default for MpdConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1:6600".to_string(),
            fifo_path: "/tmp/mpd.fifo".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub mpd: MpdConfig,
    pub simulation: SimulationParams,
}

impl AppConfig {
    pub fn load_or_create() -> Self {
        let config_dir = get_config_dir();
        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }
        let config_path = config_dir.join("config.toml");

        match fs::read_to_string(&config_path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!(
                        "Failed to parse config.toml at {:?}: {}. Falling back to defaults.",
                        config_path, e
                    );
                    Self::default()
                }
            },
            Err(_) => {
                // File doesn't exist or couldn't be read, create default
                let default_config = Self::default();
                if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
                    let _ = fs::write(&config_path, toml_str);
                }
                default_config
            }
        }
    }

    pub fn save(&self) {
        let config_dir = get_config_dir();
        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }
        let config_path = config_dir.join("config.toml");

        if let Ok(toml_str) = toml::to_string_pretty(self) {
            let _ = fs::write(&config_path, toml_str);
        }
    }

    pub fn get_presets_dir() -> PathBuf {
        let presets_dir = get_config_dir().join("presets");
        if !presets_dir.exists() {
            let _ = fs::create_dir_all(&presets_dir);
        }
        presets_dir
    }

    pub fn list_presets() -> Vec<String> {
        let presets_dir = Self::get_presets_dir();
        let mut presets = Vec::new();
        if let Ok(entries) = fs::read_dir(presets_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        presets.push(stem.to_string());
                    }
                }
            }
        }
        presets.sort();
        presets
    }

    pub fn load_preset(name: &str) -> Option<Self> {
        let preset_path = Self::get_presets_dir().join(format!("{}.toml", name));
        if let Ok(content) = fs::read_to_string(&preset_path) {
            toml::from_str(&content).ok()
        } else {
            None
        }
    }

    pub fn save_preset(&self, name: &str) {
        let preset_path = Self::get_presets_dir().join(format!("{}.toml", name));
        if let Ok(toml_str) = toml::to_string_pretty(self) {
            let _ = fs::write(&preset_path, toml_str);
        }
    }
}
