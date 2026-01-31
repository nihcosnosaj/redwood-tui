use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{info, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub location: LocationConfig,
    pub api: ApiConfig,
    pub ui: UiConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocationConfig {
    pub auto_gpu: bool,        // Use IP geolocation if true
    pub manual_lat: f64,       // Latitude used if auto_gpu is false
    pub manual_lon: f64,       // Longitude used if auto_gpu is false
    pub detection_radius: f64, // Radius in km for the OpenSky query
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiConfig {
    pub poll_interval_seconds: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiConfig {
    pub default_view: String, // "Dashboard" or "Spotter"
}

impl Config {
    /// Loads config.toml from the root directory.
    /// If it doesn't exist, creates a default one.
    pub fn load() -> Self {
        let config_path = "config.toml";

        if let Ok(content) = fs::read_to_string(config_path) {
            match toml::from_str(&content) {
                Ok(config) => return config,
                Err(e) => warn!("Failed to parse config.toml: {}. Using defaults.", e),
            }
        }

        // Default Configuration
        let default_config = Config {
            location: LocationConfig {
                auto_gpu: true,
                manual_lat: 37.7749,
                manual_lon: -122.4194,
                detection_radius: 50.0,
            },
            api: ApiConfig {
                poll_interval_seconds: 30,
            },
            ui: UiConfig {
                default_view: "Dashboard".to_string(),
            },
        };

        // Save default config to disk for the user to edit later
        let toml_string = toml::to_string_pretty(&default_config).unwrap();
        if fs::write(config_path, toml_string).is_err() {
            warn!("Could not write default config.toml to disk.");
        }

        info!("Loaded default configuration.");
        default_config
    }
}
