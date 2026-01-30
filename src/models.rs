use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flight {
    pub callsign: String,
    pub origin_country: String,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: f32,
    pub velocity: f32,
    pub true_track: f32,
}

#[derive(Deserialize)]
pub struct OpenSkyResponse {
    pub states: Option<Vec<Vec<serde_json::Value>>>,
}

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
        }
    }
}
