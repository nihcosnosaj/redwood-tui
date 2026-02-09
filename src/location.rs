//! User location resolution for the Redwood flight tracker.
//!
//! This module provides a single public function, [`get_current_location`],
//! which returns coordinates used as the center for the OpenSky API query.
//! Location is determined via IP geolocation (IpApi) with a fallback to
//! default coordinates on failure.

use ipgeolocate::{Locator, Service};
use tracing::{error, info, info_span, instrument, warn};
use tracing::Instrument;

const FALLBACK_COORDS: (f64, f64) = (37.7749, -122.4194);

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
    let lookup_span = tracing::info_span!(
        "location.lookup",
        service = %"IpApi",
        strategy = %"auto-detect"
    );

    async move {
        info!("initalizing automated geolocation request");
        match Locator::get("", Service::IpApi).await {  
            Ok(loc) => {
                let lat = loc.latitude.parse::<f64>();
                let lon = loc.longitude.parse::<f64>();

                match (lat, lon) {
                    (Ok(la), Ok(lo)) => {
                        info!(
                            lat = la, 
                            lon = lo, 
                            city = %loc.city, 
                            region = %loc.region,
                            "geolocation resolution successful"
                        );
                        (la, lo)
                    }
                    _ => {
                        warn!(
                            raw_lat = %loc.latitude, 
                            raw_lon = %loc.longitude, 
                            "failed to parse coordinate strings; using fallback"
                        );
                        FALLBACK_COORDS
                    }
                }
            }
            Err(e) => {
                error!(
                    error = %e, 
                    "geolocation service unavailable; check network connectivity or API rate limits"
                );
                FALLBACK_COORDS
            }
        }
    }
    .instrument(lookup_span)
    .await
}
