//! User location resolution for the Redwood flight tracker.
//!
//! This module provides a single public function, [`get_current_location`],
//! which returns coordinates used as the center for the OpenSky API query.
//! Location is determined via IP geolocation (IpApi) with a fallback to
//! default coordinates on failure.

use ipgeolocate::{Locator, Service};
use tracing::{error, info};

/// Resolves the user's approximate location via IP geolocation.
///
/// Uses the [IpApi](https://ip-api.com/) service to geolocate based on the
/// given IP address. On success, returns the reported latitude and longitude;
/// on network or service failure, logs an error and returns San Francisco
/// coordinates so the app can still run.
///
/// # Returns
///
/// A tuple `(latitude, longitude)` in decimal degrees (WGS84). For example,
/// San Francisco is approximately `(37.7749, -122.4194)`.
///
///
/// # Panics
///
/// Does not panic. Parse failures for latitude/longitude from the response
/// fall back to the same San Francisco default as on service error.
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
            error!(
                "Error using geolocation service: {}. Using San Francisco as default area.",
                e
            );
            (37.7749, -122.4194)
        }
    }
}
