use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use csv::ReaderBuilder;
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flight {
    pub callsign: String,
    pub origin_country: String,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: f32,
    pub velocity: f32,
    pub true_track: f32,
    pub origin_airport: Option<String>,
    pub destination_airport: Option<String>,
    pub operator: Option<String>,
    pub icao24: String,
    pub vertical_rate: f64,
    pub aircraft_type: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenSkyResponse {
    pub states: Option<Vec<Vec<serde_json::Value>>>,
}

// Unmarshal the vector JSON Response from API call to OpenSky
// into an instance of Flight.
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
            aircraft_type: None,
            icao24: data[0].as_str().unwrap_or("N/A").trim().to_string(),
            vertical_rate: data[11].as_f64().unwrap_or(0.0) as f64,
        }
    }
}

pub fn load_aircraft_csv(path: &str) -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    let mut rdr = match ReaderBuilder::new()
        .quote(b'\'')
        .from_path(path) 
    {
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
            let icao24 = record.get(icao24_idx).unwrap_or("").trim_matches('\'').trim().to_lowercase();

            let get_val = |idx: Option<usize>| {
                idx.and_then(|i| record.get(i))
                    .map(|s| s.trim_matches('\'').trim())
                    .filter(|s| !s.is_empty())
            };

            let operator = get_val(operator_idx).or_else(|| get_val(owner_idx)).unwrap_or("").to_string();
            let manufacturer = get_val(manufacturer_idx).unwrap_or("");
            let model = get_val(model_idx).unwrap_or("");
            let aircraft_type = format!("{} {}", manufacturer, model).trim().to_string();

            map.insert(icao24, (operator, aircraft_type));
        }
    }
    map
}