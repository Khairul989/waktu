//! On-disk JSON cache for the resolved zone + monthly prayer data (offline-first).

use crate::models::MonthlySolat;
use chrono::{Datelike, TimeZone};
use chrono_tz::Asia::Kuala_Lumpur;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const ONE_DAY_SECS: i64 = 86_400;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cache {
    pub zone: String,
    pub district: String,
    pub qibla_deg: f64,
    pub month_number: u32,
    pub monthly: MonthlySolat,
    pub fetched_at: i64,
}

impl Cache {
    /// Stale if the KL calendar month has rolled over since `fetched_at`, or
    /// more than a day has passed (belt-and-braces against clock skew).
    pub fn is_stale(&self, now: i64) -> bool {
        kl_month_of(now) != self.month_number || (now - self.fetched_at) > ONE_DAY_SECS
    }
}

fn kl_month_of(epoch_secs: i64) -> u32 {
    Kuala_Lumpur.timestamp_opt(epoch_secs, 0).unwrap().month()
}

pub fn load(path: &Path) -> Option<Cache> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save(path: &Path, cache: &Cache) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(cache)?;
    std::fs::write(path, raw)
}

/// Default cache location under the user's app-data dir (macOS).
pub fn default_cache_path() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base)
        .join("Library/Application Support/com.khairul.waktu")
        .join("cache.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DayPrayer;

    fn sample_cache(month_number: u32, fetched_at: i64) -> Cache {
        Cache {
            zone: "WLY01".to_string(),
            district: "Kuala Lumpur".to_string(),
            qibla_deg: 292.52,
            month_number,
            monthly: MonthlySolat {
                zone: "WLY01".to_string(),
                year: 2026,
                month: "JUL".to_string(),
                month_number,
                prayers: vec![DayPrayer {
                    day: 6,
                    hijri: "1448-01-21".to_string(),
                    imsak: 0,
                    fajr: 0,
                    syuruk: 0,
                    dhuha: 0,
                    dhuhr: 0,
                    asr: 0,
                    maghrib: 0,
                    isha: 0,
                }],
            },
            fetched_at,
        }
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("waktu-cache-test-{}", std::process::id()));
        let path = dir.join("cache.json");
        let cache = sample_cache(7, 1783328400);
        save(&path, &cache).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.zone, cache.zone);
        assert_eq!(loaded.month_number, 7);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stale_when_month_differs_or_older_than_a_day() {
        let cache = sample_cache(7, 1783328400); // fetched July 6, 2026 17:00 KL
        assert!(!cache.is_stale(1783328400 + 60)); // 1 min later, same month
        assert!(cache.is_stale(1783328400 + ONE_DAY_SECS + 1)); // >1 day later
        assert!(cache.is_stale(1785920400)); // a month later -> month_number differs
    }
}
