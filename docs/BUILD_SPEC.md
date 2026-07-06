# Waktu — Build Spec (menubar prayer-times app)

**Project dir:** `/Volumes/KhaiSSD/Documents/Github/personal/esolat`
**Stack:** Tauri v2, Rust core + vanilla HTML/CSS/JS view (NO npm framework, NO bundler).
**Platform:** macOS (Apple Silicon), Rust 1.88 stable.
**App name:** Waktu. Menubar-ONLY app (no dock icon).

## Product
A macOS **menubar** app showing the next prayer time for the user's location.
- **Tray title** (menubar text): next fard prayer + live countdown, e.g. `Asar · 43m`. Updates every ~30s.
- **Click tray** → borderless popover under the menubar showing today's full schedule, current waqt highlighted, hijri date, qibla bearing, zone. Auto-hides on blur.
- Design must match the approved mock (see DESIGN below).

## Data sources (both verified working, no auth)
### Primary — Waktu Solat API (GPS-native, monthly, cacheable)
- `GET https://api.waktusolat.app/v2/solat/gps/{lat}/{lng}` → resolves coords to a JAKIM zone AND returns the whole month.
- `GET https://api.waktusolat.app/v2/solat/{zone}` → same, by zone code.
- `GET https://api.waktusolat.app/zones` → 60 zones: `[{jakimCode,negeri,daerah}]`.
- Response shape:
```json
{"zone":"WLY01","year":2026,"month":"JUL","month_number":7,
 "prayers":[{"day":1,"hijri":"1448-01-15","imsak":1782855840,"fajr":...,
   "syuruk":...,"dhuha":...,"dhuhr":...,"asr":...,"maghrib":...,"isha":...}, ...]}
```
- **Prayer times are Unix timestamps in SECONDS (UTC).** Convert to local via Asia/Kuala_Lumpur (UTC+8) for display. `day` = day-of-month; pick the entry matching today.

### Authoritative cross-check + Qibla — JAKIM e-Solat
- `GET https://www.e-solat.gov.my/index.php?r=esolatApi/TakwimSolat&period=today&zone={ZONE}` (no cookies needed).
- Returns HH:MM:SS local strings + `bearing` (qibla, e.g. `292&#176; 31&#8242; 16&#8243;` = 292°31'16"). Use this endpoint to fetch the **qibla bearing** for the zone (parse the degrees). Times are for cross-check/fallback.

## Location strategy (v1)
1. Auto: IP geolocation — `https://ipapi.co/json/` (https, returns latitude/longitude/city); fallback `http://ip-api.com/json/`.
2. Feed lat/lng → waktusolat GPS endpoint → zone + times.
3. **Manual override:** settings lets user pick any of the 60 zones; persisted. Override wins over IP.
Zone-granular accuracy is correct for prayer times (all points in a JAKIM zone share the same times). CoreLocation = future enhancement, out of v1 scope.

## Prayer model & next-prayer logic (pure, unit-tested)
- The 7 marks in order: **Imsak, Subuh(fajr), Syuruk, Zohor(dhuhr), Asar(asr), Maghrib, Isyak(isha)**.
- Malay display names ↔ API fields: Imsak=imsak, Subuh=fajr, Syuruk=syuruk, Zohor=dhuhr, Asar=asr, Maghrib=maghrib, Isyak=isha. (dhuha not shown.)
- **Fard prayers** (drive the tray countdown + highlight): Subuh, Zohor, Asar, Maghrib, Isyak.
- `next_prayer(now, today_marks, tomorrow_subuh) -> {name, at, countdown}`: next fard whose time > now; after Isyak, roll to tomorrow's Subuh. **Must be a pure fn with unit tests** using fixture timestamps.
- "Current waqt" = the fard prayer whose window contains `now` (for list highlight).

## Caching (offline-first)
- Persist the monthly waktusolat payload + resolved zone + qibla to the Tauri app-data dir (JSON).
- Refresh when: month changes, zone changes, or cache older than 1 day. Compute today/next from cache so the app works offline and doesn't hammer the API.

## Menubar tuning (CRITICAL — this is a menubar app)
- `activation_policy(Accessory)` → no dock icon, menubar-only.
- Tray: `TrayIconBuilder` with a template PNG icon + dynamic `set_title(...)`. Left-click toggles the popover.
- Popover window: `decorations:false`, `transparent:true`, `always_on_top:true`, `skip_taskbar:true`, `resizable:false`, fixed ~360px wide, hidden by default, **auto-hide on blur** (listen for focus-lost → hide).
- Position under the tray icon — use `tauri-plugin-positioner` v2 (`TrayCenter`) or manual positioning from the tray-click event.
- Background tokio interval (~30s) recomputes countdown → `tray.set_title(...)` + emits `state-updated` event to the frontend.

## DESIGN (match the approved mock exactly)
Mock artifact: nocturnal "hour-before-Fajr" theme.
- **Palette (dark):** ground `#0B201C`/`#0E2823`/`#091B18`; text `#ECE6D4`; dim `#8FA69C`; faint `#5E7169`; gold `#C9A34E`→`#E6CB78` (active only); hairline `#22493F`. Light theme = dawn parchment (`#F8F4EA` ground, `#241E12` text, gold `#9A7521`). Honor `prefers-color-scheme` AND macOS appearance.
- **Type:** display serif (`ui-serif, "Iowan Old Style", Palatino, Georgia`) for prayer name + countdown; `system-ui` for labels/data with `tabular-nums`; system Arabic stack (`"SF Arabic","Geeza Pro"`) for Arabic names.
- **Layout (top→bottom):** hero (eyebrow "Seterusnya · Next Prayer", big serif name, Arabic below in gold, pill countdown `in 43 min · 4:46 PM`) → date strip (Malay gregorian left, hijri w/ crescent right) → 7-row prayer list (dot tick + Malay name + small Arabic + time; passed rows dimmed; active row gold left-bar, gold text, glowing pulsing dot) → footer (Kiblat bearing + Zon code·district + settings gear).
- Girih 8-point star field faintly behind the hero (canvas, ~10% gold). Subtle, atmospheric.
- Arabic + Malay names per row: Imsak إمساك · Subuh الفجر · Syuruk الشروق · Zohor الظهر · Asar العصر · Maghrib المغرب · Isyak العشاء.
- Respect `prefers-reduced-motion`. Popover corners rounded 18px, soft shadow, top notch pointing to menubar.

## Frontend ↔ Rust commands (Tauri `invoke`)
- `get_state()` → `{zone, district, hijri, gregorian, qibla_deg, prayers:[{key,name,arabic,at_ms,passed,is_active}], next:{name,at_ms}}`
- `list_zones()` → the 60 zones for the settings picker.
- `set_zone(code)` → override + refresh + return new state.
- `refresh()` → force re-fetch.
- Frontend runs its own 1s countdown tick from `next.at_ms`; re-pulls `get_state()` on `state-updated` event.

## Proving commands
- Wave 1: `cd esolat/src-tauri && cargo build` (green) + `cargo test` (next-prayer unit tests pass).
- Wave 2: `cd esolat && cargo tauri build` (green) — app bundle produced; manual run shows tray countdown + popover with real data matching the mock.
