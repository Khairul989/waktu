//! Resolves the user's approximate lat/lng via IP geolocation (no GPS/CoreLocation in v1).

use serde_json::Value;
use std::time::Duration;

use crate::api::ApiError;

fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
}

/// Resolves `(lat, lng)` via ipapi.co, falling back to ip-api.com on failure.
pub async fn resolve_latlng() -> Result<(f64, f64), ApiError> {
    if let Ok(coords) = try_ipapi_co().await {
        return Ok(coords);
    }
    try_ip_api_com().await
}

async fn try_ipapi_co() -> Result<(f64, f64), ApiError> {
    let body: Value = client()?
        .get("https://ipapi.co/json/")
        .send()
        .await?
        .json()
        .await?;
    let lat = body.get("latitude").and_then(Value::as_f64).ok_or("no latitude")?;
    let lng = body.get("longitude").and_then(Value::as_f64).ok_or("no longitude")?;
    Ok((lat, lng))
}

async fn try_ip_api_com() -> Result<(f64, f64), ApiError> {
    let body: Value = client()?
        .get("http://ip-api.com/json/")
        .send()
        .await?
        .json()
        .await?;
    let lat = body.get("lat").and_then(Value::as_f64).ok_or("no lat")?;
    let lng = body.get("lon").and_then(Value::as_f64).ok_or("no lon")?;
    Ok((lat, lng))
}
