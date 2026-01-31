use ipgeolocate::{Locator, Service};
use tracing::{info, error};

pub async fn get_current_location() -> (f64, f64) {
    // Using IpApi as the service, it's pretty reliable.
    match Locator::get("1.1.1.1", Service::IpApi).await {
        Ok(loc) => {
            let lat = loc.latitude.parse::<f64>().unwrap_or(37.7749);
            let lon = loc.longitude.parse::<f64>().unwrap_or(-122.4194);
            info!("Geolocation successful - ({}, {})", lat, lon);
            (lat, lon)
        }
        Err(e) => {
            // Use SF as a default if lookup fails.
            error!("Error using geolocation service: {}. Using San Francisco as default area.", e);
            (37.7749, -122.4194)
        }
    }
}