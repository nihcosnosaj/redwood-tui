//! Data models and parsing for the Redwood flight tracker.
//!
//! This module defines:
//! - **[`Flight`]** — A single aircraft’s state (position, identity, telemetry), populated from
//!   the OpenSky API and optionally enriched by the local aircraft database.
//! - **[`OpenSkyResponse`]** — Raw JSON response shape from the OpenSky “states” API.
//! - **Conversion** from OpenSky’s state-vector format into [`Flight`] via [`From`].
//! - **[`load_aircraft_csv`]** — Builds a lookup map (ICAO24 → operator/type) from the
//!   aircraft CSV; the DB layer uses this data when building the SQLite DB.

use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::error;

/// A single aircraft’s current state and identity.
///
/// Core fields come from the [OpenSky state vector](https://opensky-network.org/docs/api/v1.html#response)
/// (e.g. position, altitude, velocity). Optional fields are filled when the
/// aircraft is found in the local aircraft database (see `db::decorate_flights`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Flight {
    /// Flight or operator callsign (e.g. "UAL123")
    pub callsign: String,
    /// Country of registration/origin.
    pub origin_country: String,
    /// Longitude in decimal degrees.
    pub longitude: f64,
    /// Latitude in decimal degrees.
    pub latitude: f64,
    /// Altitude in meters.
    pub altitude: f32,
    /// Velocity in meters per second.
    pub velocity: f32,
    /// True track in degrees.
    pub true_track: f32,
    /// Origin airport (ICAO code).
    pub origin_airport: Option<String>,
    /// Destination airport (ICAO code).
    pub destination_airport: Option<String>,
    /// Operator (e.g. "United Airlines").
    pub operator: Option<String>,
    /// ICAO24 aircraft identifier.
    pub icao24: String,
    /// Vertical rate in meters per second.
    pub vertical_rate: f64,
    /// Aircraft type (e.g. "Boeing 747").
    pub aircraft_type: Option<String>,
    /// Operator callsign (e.g. "UAL").
    pub operator_callsign: Option<String>,
    /// Manufacturer (e.g. "Boeing").
    pub manufacturer: Option<String>,
    /// Model (e.g. "747").
    pub model: Option<String>,
    /// Registration (e.g. "N12345").
    pub registration: Option<String>,
}

/// Raw response from the OpenSky Network “states/all” (or bounding-box) API.
///
/// `states` is an optional array of state vectors. Each vector is an array of
/// [`serde_json::Value`]s whose indices follow the [OpenSky state vector format](https://opensky-network.org/docs/api/v1.html#response).
#[derive(Deserialize)]
pub struct OpenSkyResponse {
    pub states: Option<Vec<Vec<serde_json::Value>>>,
}

/// Builds a [`Flight`] from a single OpenSky state vector.
///
/// Indices follow the [OpenSky API state vector](https://opensky-network.org/docs/api/v1.html#response):
/// 0 = icao24, 1 = callsign, 2 = origin_country, 5 = longitude, 6 = latitude,
/// 7 = altitude, 9 = velocity, 10 = true_track, 11 = vertical_rate. Fields not
/// provided by the API (operator, registration, etc.) are set to `None` and
/// can be filled later by `db::decorate_flights`.
///
/// # Panics
///
/// Does not panic; missing or invalid values use defaults (e.g. 0.0 for numbers,
/// "N/A" or "Unknown" for strings).
///
/// # Arguments
///
/// * `data` - A vector of [`serde_json::Value`]s representing the OpenSky state vector.
///
/// # Returns
///
/// A [`Flight`] struct populated with the data from the OpenSky state vector.
///
/// # Panics
///
/// Does not panic; missing or invalid values use defaults (e.g. 0.0 for numbers,
/// "N/A" or "Unknown" for strings).
impl From<Vec<serde_json::Value>> for Flight {
    fn from(data: Vec<serde_json::Value>) -> Self {
        Self {
            callsign: data[1].as_str().unwrap_or("N/A").trim().to_string(),
            origin_country: data[2].as_str().unwrap_or("Unknown").to_string(),
            longitude: data[5].as_f64().unwrap_or(0.0),
            latitude: data[6].as_f64().unwrap_or(0.0),
            altitude: data[7].as_f64().unwrap_or(0.0) as f32,
            velocity: data[9].as_f64().unwrap_or(0.0) as f32,
            true_track: data[10].as_f64().unwrap_or(0.0) as f32,
            origin_airport: None,
            destination_airport: None,
            operator: None,
            icao24: data[0].as_str().unwrap_or("N/A").trim().to_string(),
            vertical_rate: data[11].as_f64().unwrap_or(0.0),
            operator_callsign: None,
            manufacturer: None,
            model: None,
            registration: None,
            aircraft_type: None,
        }
    }
}

impl Flight {
    /// Great-circle distance from this flight's position to a point.
    ///
    /// Uses the [Haversine formula](https://en.wikipedia.org/wiki/Haversine_formula)
    /// with Earth's radius 6371 km. Suitable for "nearby" distances.
    ///
    /// # Arguments
    ///
    /// * `user_lat` - Observer latitude in decimal degrees.
    /// * `user_lon` - Observer longitude in decimal degrees.
    ///
    /// # Returns
    ///
    /// Distance in kilometers.
    pub fn distance_from(&self, user_lat: f64, user_lon: f64) -> f64 {
        let r = 6371.0; // Earth's radius in km

        // Convert everything to radians
        let lat1 = user_lat.to_radians();
        let lon1 = user_lon.to_radians();
        let lat2 = self.latitude.to_radians();
        let lon2 = self.longitude.to_radians();

        let d_lat = lat2 - lat1;
        let d_lon = lon2 - lon1;

        let a = (d_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        r * c
    }
}

/// Loads the aircraft CSV into a map keyed by ICAO24.
///
/// Expects a CSV with single-quoted fields and headers including `icao24`.
/// Optional columns (matched case-insensitively): `operator`, `owner`,
/// `manufacturername`, `model`. Operator is taken from `operator` or `owner`.
/// The value is `(operator, aircraft_type)` where `aircraft_type` is
/// `"manufacturer model"` (trimmed).
///
/// # Arguments
///
/// * `path` - Path to the CSV file (e.g. `data/aircraft-database-complete-2025-08.csv`).
///
/// # Returns
///
/// A map from lowercase ICAO24 string to `(operator, aircraft_type)`.
/// On open/parse errors or missing `icao24` header, logs and returns an empty map.
pub fn load_aircraft_csv(path: &str) -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    let mut rdr = match ReaderBuilder::new().quote(b'\'').from_path(path) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to load aircraft database from '{}': {}", path, e);
            return map;
        }
    };

    let headers = match rdr.headers() {
        Ok(h) => h.clone(),
        Err(e) => {
            error!("Failed to read CSV headers: {}", e);
            return map;
        }
    };

    // Helper to find column index case-insensitively
    let find_col = |name: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(name));

    let icao24_idx = match find_col("icao24") {
        Some(i) => i,
        None => {
            error!("CSV missing 'icao24' column. Headers found: {:?}", headers);
            return map;
        }
    };

    let operator_idx = find_col("operator");
    let owner_idx = find_col("owner");
    let manufacturer_idx = find_col("manufacturername");
    let model_idx = find_col("model");

    for result in rdr.records() {
        if let Ok(record) = result {
            let icao24 = record
                .get(icao24_idx)
                .unwrap_or("")
                .trim_matches('\'')
                .trim()
                .to_lowercase();

            let get_val = |idx: Option<usize>| {
                idx.and_then(|i| record.get(i))
                    .map(|s| s.trim_matches('\'').trim())
                    .filter(|s| !s.is_empty())
            };

            let operator = get_val(operator_idx)
                .or_else(|| get_val(owner_idx))
                .unwrap_or("")
                .to_string();
            let manufacturer = get_val(manufacturer_idx).unwrap_or("");
            let model = get_val(model_idx).unwrap_or("");
            let aircraft_type = format!("{} {}", manufacturer, model).trim().to_string();

            map.insert(icao24, (operator, aircraft_type));
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_calculation() {
        let flight = Flight {
            latitude: 37.7749, // San Francisco
            longitude: -122.4194,
            ..Default::default()
        };

        // 1. Test Identity (Distance to self = 0)
        let dist = flight.distance_from(37.7749, -122.4194);
        assert!(
            dist < 0.01,
            "Distance to self should be near zero, got {}",
            dist
        );

        // 2. Test SF to Oakland (Approx 14km)
        // Oakland Coords: 37.8044, -122.2712
        let dist_oak = flight.distance_from(37.8044, -122.2712);

        // Use a more realistic range or a delta check
        assert!(
            dist_oak > 13.0 && dist_oak < 15.0,
            "SF to Oakland should be ~14km, got {:.2}km",
            dist_oak
        );
    }
}
