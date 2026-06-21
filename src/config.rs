use serde::{Deserialize, Serialize};
use std::fs;
use bevy::prelude::Resource;

use crate::SimulationParams;

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
        let config_path = "config.toml";
        
        match fs::read_to_string(config_path) {
            Ok(content) => {
                match toml::from_str(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Failed to parse config.toml: {}. Falling back to defaults.", e);
                        Self::default()
                    }
                }
            }
            Err(_) => {
                // File doesn't exist or couldn't be read, create default
                let default_config = Self::default();
                if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
                    let _ = fs::write(config_path, toml_str);
                }
                default_config
            }
        }
    }

    pub fn save(&self) {
        let config_path = "config.toml";
        if let Ok(toml_str) = toml::to_string_pretty(self) {
            let _ = fs::write(config_path, toml_str);
        }
    }
}
