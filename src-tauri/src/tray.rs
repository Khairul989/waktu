//! Tray icon: dynamic `<Name> · <countdown>` title, left-click toggles the
//! popover, and a 30s background loop keeps both the title and the popover
//! (via `state-updated`) fresh without the frontend polling the backend.

use chrono::TimeZone;
use chrono_tz::Asia::Kuala_Lumpur;
use objc2_app_kit::{NSEvent, NSScreen, NSWindow};
use objc2_foundation::{MainThreadMarker, NSPoint};
use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_nspanel::ManagerExt;

use crate::commands::compute_state;
use crate::models::AppState;

const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray.png");
const TRAY_ID: &str = "waktu-tray";

/// Fard prayer keys — the ones that trigger a notification (excludes syuruk).
const FARD_KEYS: [&str; 5] = ["fajr", "dhuhr", "asr", "maghrib", "isha"];

/// Formats epoch-millis as a KL-local `h:mm AM/PM` time, e.g. "4:46 PM".
fn format_kl_time(at_ms: i64) -> String {
    let dt = Kuala_Lumpur.timestamp_millis_opt(at_ms).unwrap();
    // `%-I` (no leading zero) isn't portable everywhere; strip a leading "0" manually instead.
    let s = dt.format("%I:%M %p").to_string();
    s.strip_prefix('0').unwrap_or(&s).to_string()
}

/// Sends a desktop notification via `osascript` (`display notification`). The
/// Tauri notification plugin silently fails for unsigned apps on recent macOS,
/// while osascript delivers reliably — so we shell out to it here.
fn notify(title: &str, body: &str) {
    // Escape backslash and double-quote so the values can't break out of the
    // AppleScript string literal (district/name come from an external API).
    fn esc(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"")
    }
    let script = format!(
        "display notification \"{}\" with title \"{}\" sound name \"Glass\"",
        esc(body),
        esc(title)
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();
}

/// If a fard prayer has just become due (arrived within the last 2 minutes,
/// and not already notified), sends a native reminder notification.
fn notify_if_due(state: &AppState, last_notified_ms: &mut i64) {
    let now = now_ms();
    // 2-minute guard: without it, a prayer that passed long before this tick
    // (e.g. app just launched hours after Subuh) would fire a stale alert.
    let Some(prayer) = state
        .prayers
        .iter()
        .filter(|p| FARD_KEYS.contains(&p.key.as_str()) && p.at_ms <= now)
        .max_by_key(|p| p.at_ms)
    else {
        return;
    };
    if prayer.at_ms <= *last_notified_ms || now - prayer.at_ms >= 120_000 {
        return;
    }

    let body = format!(
        "Telah masuk waktu {} ({}) di {}.",
        prayer.name,
        format_kl_time(prayer.at_ms),
        state.district
    );
    notify(&format!("Waktu {}", prayer.name), &body);
    *last_notified_ms = prayer.at_ms;
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// `43m` under an hour, `2h 05m` at/over an hour.
fn format_countdown(at_ms: i64, now_ms: i64) -> String {
    let remaining_min = (at_ms - now_ms).max(0) / 60_000;
    if remaining_min < 60 {
        format!("{remaining_min}m")
    } else {
        format!("{}h {:02}m", remaining_min / 60, remaining_min % 60)
    }
}

/// Builds the tray icon (template PNG) and wires left-click to toggle the popover.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let icon = Image::from_bytes(TRAY_ICON_BYTES)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app_handle = tray.app_handle();
                let Ok(panel) = app_handle.get_webview_panel("main") else {
                    return;
                };

                if panel.is_visible() {
                    panel.order_out(None);
                    return;
                }

                if let Some(window) = app_handle.get_webview_window("main") {
                    position_popover(&window);
                }
                panel.show();
                // The webview is suspended (App Nap) while the panel is hidden,
                // so it misses `state-updated` events and would otherwise show
                // stale data (e.g. yesterday's) on open. Tell it to re-fetch.
                let _ = app_handle.emit("popover-shown", ());
            }
        })
        .build(app)?;
    Ok(())
}

/// Positions the popover natively via AppKit instead of Tauri's tray-event
/// coordinates: `TrayIconEvent` reports the click in PHYSICAL pixels (points
/// times the clicked display's scale factor), which makes cross-monitor math
/// fragile. `NSEvent::mouseLocation()` + `NSScreen::screens()` share one
/// unified points-based global space, so placement needs zero scale-factor
/// math.
fn position_popover(window: &tauri::WebviewWindow) {
    let mtm = MainThreadMarker::new().expect("tray event on main thread");
    let mouse = NSEvent::mouseLocation();

    let screens = NSScreen::screens(mtm);
    let screen = screens
        .iter()
        .find(|s| {
            let f = s.frame();
            mouse.x >= f.origin.x
                && mouse.x < f.origin.x + f.size.width
                && mouse.y >= f.origin.y
                && mouse.y < f.origin.y + f.size.height
        })
        .or_else(|| NSScreen::mainScreen(mtm));

    let Some(screen) = screen else {
        return;
    };
    let vf = screen.visibleFrame();

    let Ok(ns_ptr) = window.ns_window() else {
        return;
    };
    let ns_window: &NSWindow = unsafe { &*(ns_ptr as *mut NSWindow) };
    let w = ns_window.frame().size.width;

    let top_y = vf.origin.y + vf.size.height; // top of visible area, just under the menubar
    let mut x = mouse.x - w / 2.0; // center under the mouse/icon
    x = x
        .max(vf.origin.x + 8.0)
        .min(vf.origin.x + vf.size.width - w - 8.0);
    let top_left = NSPoint::new(x, top_y);

    ns_window.setFrameTopLeftPoint(top_left);
}

/// Every 30s: recompute state, push it into the tray title, and emit
/// `state-updated` so an open popover re-renders (day/month rollover is just
/// `compute_state()` naturally refreshing the cache when it goes stale).
pub fn spawn_updater(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_notified_ms: i64 = 0;
        loop {
            if let Ok(state) = compute_state().await {
                notify_if_due(&state, &mut last_notified_ms);

                if let Some(tray) = app.tray_by_id(TRAY_ID) {
                    let title = format!(
                        "{} · {}",
                        state.next.name,
                        format_countdown(state.next.at_ms, now_ms())
                    );
                    let _ = tray.set_title(Some(title));
                }
                let _ = app.emit("state-updated", &state);
            }
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });
}
