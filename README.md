# Waktu 🕌

A macOS **menubar** app (Rust / Tauri v2) that shows the next prayer time for your location, sourced live from JAKIM. Designed menubar-first: no dock icon, a live countdown in the menu bar, and a nocturnal "hour-before-Fajr" popover with Islamic geometry.

![menubar-first prayer companion](docs/BUILD_SPEC.md)

## Features
- **Next prayer + live countdown** right in the menu bar (e.g. `Asar · 43m`).
- Click the tray → popover with today's full schedule (Imsak → Isyak), the current waqt gold-lit, the **Hijri date**, and the **qibla bearing** for your zone.
- **Auto location** via IP geolocation → resolved to a JAKIM zone; **manual zone override** (all 60 zones) in settings.
- **Offline-first**: a whole month is cached, so times show even without a connection.
- Bilingual Malay + Arabic prayer names; light & dark themes follow macOS appearance.

## Data sources
- **Primary — [Waktu Solat API](https://api.waktusolat.app/docs)**: `GET /v2/solat/gps/{lat}/{lng}` resolves coordinates to a zone and returns the month; `GET /zones` lists all zones. (Sources its data from JAKIM.)
- **Qibla + cross-check — JAKIM e-Solat**: `GET e-solat.gov.my/index.php?r=esolatApi/TakwimSolat&period=today&zone={ZONE}` (provides the qibla `bearing`).

See `docs/BUILD_SPEC.md` for the full architecture and API notes.

## Download & install
Grab the latest `.dmg` from the [Releases page](https://github.com/Khairul989/waktu/releases), open it, drag **Waktu** to Applications, and launch. The app is signed with a Developer ID and notarized by Apple, so it opens without security prompts.

(Menubar-only — look for it in the menu bar, not the Dock.)

## Develop
```sh
# prerequisites: Rust stable, and the Tauri CLI v2
cargo install tauri-cli --version "^2" --locked

# run the app (live)
cargo tauri dev

# or just the Rust core
cd src-tauri
cargo build          # compile
cargo test           # 9 unit tests (next-prayer logic, cache, config, bearing parse)
```

## Build a release bundle
```sh
cargo tauri build    # produces Waktu.app under src-tauri/target/release/bundle/
```

## Project layout
```
src/                 vanilla HTML/CSS/JS popover (no framework/bundler)
src-tauri/src/
  api.rs             waktusolat + e-solat HTTP clients
  location.rs        IP geolocation (ipapi.co → ip-api.com fallback)
  prayer.rs          pure next-prayer / active-waqt logic (unit-tested)
  cache.rs           offline monthly cache (app-data dir)
  config.rs          manual-zone override
  commands.rs        Tauri commands (get_state / list_zones / set_zone / clear_zone / refresh)
  tray.rs            menubar tray title + 30s updater
  lib.rs             Accessory policy, tray wiring, blur-hide
docs/BUILD_SPEC.md   architecture + API reference
```
