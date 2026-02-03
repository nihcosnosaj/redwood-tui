//! Configuration loading and defaults for the Redwood flight tracker.
//!
//! Configuration is read from `config.toml` in the current working directory.
//! If the file is missing or invalid, defaults are used and a default file is
//! written so the user can edit it. See [`Config::load`].

use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{info, warn};

/// Path to the configuration file (current working directory).
const CONFIG_PATH: &str = "config.toml";

/// Root configuration structure; maps to the top level of `config.toml`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// Location and detection radius settings.
    pub location: LocationConfig,
    /// API poll interval.
    pub api: ApiConfig,
    /// UI defaults.
    pub ui: UiConfig,
}

/// Location source and search radius for the OpenSky API.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocationConfig {
    /// If `true`, use IP geolocation for the user's position; if `false`, use
    /// `manual_lat` and `manual_lon`. (Name preserved for config file compatibility.)
    pub auto_gpu: bool,
    /// Manual latitude in decimal degrees. Used when `auto_gpu` is `false`.
    pub manual_lat: f64,
    /// Manual longitude in decimal degrees. Used when `auto_gpu` is `false`.
    pub manual_lon: f64,
    /// Search radius in kilometres for the OpenSky bounding-box query.
    pub detection_radius: f64,
}

/// API-related settings.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiConfig {
    /// Seconds between OpenSky API fetches.
    pub poll_interval_seconds: u64,
}

/// UI-related settings.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiConfig {
    /// Initial view: `"Dashboard"` or `"Spotter"`. Any other value falls back to Spotter.
    pub default_view: String,
}

impl Default for LocationConfig {
    fn default() -> Self {
        Self {
            auto_gpu: true,
            manual_lat: 37.7749,
            manual_lon: -122.4194,
            detection_radius: 50.0,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            poll_interval_seconds: 30,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            default_view: "Dashboard".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            location: LocationConfig::default(),
            api: ApiConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Config {
    /// Loads configuration from `config.toml` in the current working directory.
    ///
    /// If the file exists and parses successfully, returns the parsed config.
    /// If the file is missing or parsing fails, returns [`Config::default`],
    /// writes the default config to `config.toml` (log a warning on write failure),
    /// and logs that defaults were loaded.
    ///
    /// # Returns
    ///
    /// A valid [`Config`]; never fails. Missing or invalid files result in defaults.
    ///
    /// # Panics
    ///
    /// Does not panic. Serialization of the default config for writing is
    /// infallible for the current struct layout.
    pub fn load() -> Self {
        if let Ok(content) = fs::read_to_string(CONFIG_PATH) {
            if let Ok(config) = toml::from_str(&content) {
                return config;
            }
            warn!("Failed to parse config.toml. Using defaults.");
        }

        let default_config = Config::default();
        if let Ok(toml_string) = toml::to_string_pretty(&default_config) {
            if fs::write(CONFIG_PATH, toml_string).is_err() {
                warn!("Could not write default config.toml to disk.");
            }
        } else {
            warn!("Could not serialize default config.");
        }

        info!("Loaded default configuration.");
        default_config
    }
}
