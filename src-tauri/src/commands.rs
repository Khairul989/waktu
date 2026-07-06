//! Tauri commands: orchestrate config -> cache -> api -> prayer, in-process (no
//! shared Tauri state) since each command just loads/saves the on-disk cache.

use chrono::{Datelike, TimeZone};
use chrono_tz::Asia::Kuala_Lumpur;

use crate::api::{self, ApiError};
use crate::cache::{self, Cache};
use crate::config::{self, Config};
use crate::location;
use crate::models::{AppState, Zone};
use crate::prayer;

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn kl_day_of(now: i64) -> u32 {
    Kuala_Lumpur.timestamp_opt(now, 0).unwrap().day()
}

fn kl_gregorian_string(now: i64) -> String {
    Kuala_Lumpur
        .timestamp_opt(now, 0)
        .unwrap()
        .format("%A, %d %B %Y")
        .to_string()
}

/// Builds the frontend-facing state from a cached monthly payload + `now`.
fn build_state(cache: &Cache, now: i64) -> AppState {
    let today_dom = kl_day_of(now);
    let today = cache
        .monthly
        .prayers
        .iter()
        .find(|p| p.day == today_dom)
        .or_else(|| cache.monthly.prayers.last())
        .expect("monthly payload has at least one day");
    // Month-boundary edge case: no "tomorrow" entry in this payload — 24h
    // past today's fajr is close enough for the countdown to roll over on.
    let tomorrow_fajr = cache
        .monthly
        .prayers
        .iter()
        .find(|p| p.day == today_dom + 1)
        .map(|d| d.fajr)
        .unwrap_or(today.fajr + 86_400);

    AppState {
        zone: cache.zone.clone(),
        district: cache.district.clone(),
        hijri: today.hijri.clone(),
        gregorian: kl_gregorian_string(now),
        qibla_deg: cache.qibla_deg,
        prayers: prayer::build_prayer_rows(today, tomorrow_fajr, now),
        next: prayer::next_prayer(today, tomorrow_fajr, now),
    }
}

async fn resolve_district(zone: &str) -> Result<String, ApiError> {
    let zones = api::fetch_zones().await?;
    zones
        .into_iter()
        .find(|z| z.jakim_code == zone)
        .map(|z| z.daerah)
        .ok_or_else(|| "zone not found in zones list".into())
}

/// Fetches fresh monthly data (by manual zone, or GPS-resolved) + qibla bearing.
async fn refresh_state(config: &Config) -> Result<Cache, ApiError> {
    let monthly = match &config.manual_zone {
        Some(zone) => api::fetch_by_zone(zone).await?,
        None => {
            let (lat, lng) = location::resolve_latlng().await?;
            api::fetch_by_gps(lat, lng).await?
        }
    };
    let zone = monthly.zone.clone();
    let district = resolve_district(&zone)
        .await
        .unwrap_or_else(|_| zone.clone());
    // Qibla is a nice-to-have cross-check fetch — don't fail the whole refresh over it.
    let qibla_deg = api::fetch_qibla(&zone).await.unwrap_or(0.0);
    let month_number = monthly.month_number;

    Ok(Cache {
        zone,
        district,
        qibla_deg,
        month_number,
        monthly,
        fetched_at: now_epoch(),
    })
}

/// Loads the cache if fresh, else refreshes; falls back to a stale cache on
/// network failure (offline-first).
async fn get_or_refresh_cache(config: &Config) -> Result<Cache, String> {
    let now = now_epoch();
    let path = cache::default_cache_path();
    let existing = cache::load(&path);
    let zone_changed = existing
        .as_ref()
        .and_then(|c| config.manual_zone.as_deref().map(|z| z != c.zone))
        .unwrap_or(false);
    let stale = existing
        .as_ref()
        .map(|c| c.is_stale(now) || zone_changed)
        .unwrap_or(true);

    if !stale {
        return Ok(existing.unwrap());
    }
    match refresh_state(config).await {
        Ok(fresh) => {
            let _ = cache::save(&path, &fresh);
            Ok(fresh)
        }
        Err(e) => existing.ok_or_else(|| format!("refresh failed and no cache available: {e}")),
    }
}

/// Shared by the `get_state` command and the tray's 30s background updater
/// (Wave 2) so both paths recompute state identically.
pub async fn compute_state() -> Result<AppState, String> {
    let config = config::load(&config::default_config_path());
    let cache = get_or_refresh_cache(&config).await?;
    Ok(build_state(&cache, now_epoch()))
}

#[tauri::command]
pub async fn get_state() -> Result<AppState, String> {
    compute_state().await
}

#[tauri::command]
pub async fn list_zones() -> Result<Vec<Zone>, String> {
    api::fetch_zones().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_zone(code: String) -> Result<AppState, String> {
    let config = Config {
        manual_zone: Some(code),
    };
    config::save(&config::default_config_path(), &config).map_err(|e| e.to_string())?;
    let cache = get_or_refresh_cache(&config).await?;
    Ok(build_state(&cache, now_epoch()))
}

/// Clears the manual zone override so location falls back to IP geolocation.
/// Not in the Wave 1 command set — added for the settings "Auto" option (Wave 2 spec, section B).
#[tauri::command]
pub async fn clear_zone() -> Result<AppState, String> {
    let config = Config { manual_zone: None };
    config::save(&config::default_config_path(), &config).map_err(|e| e.to_string())?;
    let cache = get_or_refresh_cache(&config).await?;
    Ok(build_state(&cache, now_epoch()))
}

#[tauri::command]
pub async fn refresh() -> Result<AppState, String> {
    let config = config::load(&config::default_config_path());
    let fresh = refresh_state(&config).await.map_err(|e| e.to_string())?;
    cache::save(&cache::default_cache_path(), &fresh).map_err(|e| e.to_string())?;
    Ok(build_state(&fresh, now_epoch()))
}
