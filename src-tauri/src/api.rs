//! HTTP clients: waktusolat (GPS/zone/zones) + e-solat (qibla bearing cross-check).

use crate::models::{MonthlySolat, Zone};
use serde_json::Value;
use std::time::Duration;

/// Boxed error alias — avoids pulling in `anyhow` for a handful of call sites.
pub type ApiError = Box<dyn std::error::Error + Send + Sync>;

const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Waktu/0.1 (menubar prayer app)";

fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(USER_AGENT)
        .build()
}

pub async fn fetch_by_gps(lat: f64, lng: f64) -> Result<MonthlySolat, ApiError> {
    let url = format!("https://api.waktusolat.app/v2/solat/gps/{lat}/{lng}");
    Ok(client()?.get(url).send().await?.json().await?)
}

pub async fn fetch_by_zone(zone: &str) -> Result<MonthlySolat, ApiError> {
    let url = format!("https://api.waktusolat.app/v2/solat/{zone}");
    Ok(client()?.get(url).send().await?.json().await?)
}

pub async fn fetch_zones() -> Result<Vec<Zone>, ApiError> {
    let url = "https://api.waktusolat.app/zones";
    Ok(client()?.get(url).send().await?.json().await?)
}

/// Fetches the qibla bearing (decimal degrees) for a zone from the JAKIM
/// e-Solat TakwimSolat endpoint, e.g. `"292&#176; 31&#8242; 16&#8243;"` -> `292.52`.
pub async fn fetch_qibla(zone: &str) -> Result<f64, ApiError> {
    let url = format!(
        "https://www.e-solat.gov.my/index.php?r=esolatApi/TakwimSolat&period=today&zone={zone}"
    );
    let body: Value = client()?.get(url).send().await?.json().await?;
    let raw = body
        .get("bearing")
        .and_then(Value::as_str)
        .ok_or("missing bearing field")?;
    parse_bearing(raw).ok_or_else(|| format!("unparseable bearing: {raw}").into())
}

/// Parses a `D° M' S"` bearing (HTML-entity encoded) into decimal degrees.
fn parse_bearing(raw: &str) -> Option<f64> {
    let cleaned = raw
        .replace("&#176;", " ")
        .replace("&#8242;", " ")
        .replace("&#8243;", " ");
    let nums: Vec<f64> = cleaned
        .split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    match nums.as_slice() {
        [d, m, s] => Some(d + m / 60.0 + s / 3600.0),
        [d, m] => Some(d + m / 60.0),
        [d] => Some(*d),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_degrees_minutes_seconds_bearing() {
        let bearing = parse_bearing("292&#176; 31&#8242; 16&#8243;").unwrap();
        assert!((bearing - 292.521_111).abs() < 0.001);
    }
}
