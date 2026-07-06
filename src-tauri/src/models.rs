//! Serde data models: waktusolat API shapes + frontend-facing view models.

use serde::{Deserialize, Serialize};

/// One JAKIM zone, as returned by `GET /zones`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    #[serde(rename = "jakimCode")]
    pub jakim_code: String,
    pub negeri: String,
    pub daerah: String,
}

/// One day's prayer times, epoch seconds (UTC) per the waktusolat API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayPrayer {
    pub day: u32,
    pub hijri: String,
    pub imsak: i64,
    pub fajr: i64,
    pub syuruk: i64,
    pub dhuha: i64,
    pub dhuhr: i64,
    pub asr: i64,
    pub maghrib: i64,
    pub isha: i64,
}

/// A full month's worth of prayer times for a zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySolat {
    pub zone: String,
    pub year: i32,
    pub month: String,
    pub month_number: u32,
    pub prayers: Vec<DayPrayer>,
}

/// One row in the frontend's 7-row prayer list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrayerRow {
    pub key: String,
    pub name: String,
    pub arabic: String,
    pub at_ms: i64,
    pub passed: bool,
    pub is_active: bool,
}

/// The next upcoming fard prayer, driving the tray countdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextPrayer {
    pub name: String,
    pub at_ms: i64,
}

/// Full state returned by `get_state`/`refresh`/`set_zone` to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub zone: String,
    pub district: String,
    pub hijri: String,
    pub gregorian: String,
    pub qibla_deg: f64,
    pub prayers: Vec<PrayerRow>,
    pub next: NextPrayer,
}
