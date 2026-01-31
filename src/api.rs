use crate::models::{Flight, OpenSkyResponse};
use color_eyre::Result;
use reqwest::Client;

pub struct FlightProvider {
    client: Client,
}

impl Default for FlightProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FlightProvider {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_overhead(&self, lat: f64, lon: f64, radius_km: f64) -> Result<Vec<Flight>> {
        // convert KM radius to approx decimal degree.
        // 1 degree is roughly 111 KM
        let padding = radius_km / 111.0;
        let url = format!(
            "https://opensky-network.org/api/states/all?lamin={}&lomin={}&lamax={}&lomax={}",
            lat - padding,
            lon - padding,
            lat + padding,
            lon + padding
        );

        let res = self
            .client
            .get(url)
            .send()
            .await?
            .json::<OpenSkyResponse>()
            .await?;

        let flights = res
            .states
            .unwrap_or_default()
            .into_iter()
            .map(Flight::from)
            .collect();

        Ok(flights)
    }
}
