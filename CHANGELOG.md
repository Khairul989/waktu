# Changelog

All notable changes to Waktu are documented here.

## [0.2.3] - 2026-07-14

### Fixed
- Popover no longer freezes on a stale day-old state after the app is App-Napped
  overnight (e.g. showing yesterday's date with the countdown stuck at "0 min").
  The `popover-shown` event can be lost across the webview's suspend/resume, so
  the frontend now self-heals two ways: the countdown re-fetches whenever it
  reaches zero (a fresh `next` prayer is always in the future, so `0` means the
  state is stale), and a native `visibilitychange` listener re-fetches whenever
  the panel becomes visible.

## [0.2.2] - 2026-07-07

### Fixed
- Distribution: the app bundle is now ad-hoc signed (`signingIdentity: "-"`), so a
  downloaded build no longer shows "'Waktu' is damaged and can't be opened". It now
  falls under the normal unsigned-app flow (right-click → Open, or `xattr -cr`).

## [0.2.1] - 2026-07-07

### Fixed
- Popover no longer shows stale data after a day rollover — it re-fetches fresh
  state every time it's opened (the webview is suspended while the panel is
  hidden and was missing `state-updated` events).
- Active-waqt highlight is no longer missing during the Duha gap (after sunrise,
  before Zohor): when no fard window is current, the next upcoming prayer is
  highlighted instead.

## [0.2.0] - 2026-07-06

### Added
- Prayer-time notifications: a native "Waktu <prayer>" alert with sound fires
  when each fard prayer comes in (delivered via `osascript` for reliable
  delivery on unsigned builds).
- App icon: teal squircle with a gold crescent and star.

### Changed
- Popover converted to a non-activating `NSPanel` (`tauri-nspanel`): it now
  floats above all apps and Spaces, opens under the tray icon on the correct
  monitor, holds focus without stealing app activation, and dismisses on
  click-away. Fixes the popover appearing on the wrong monitor / behind the
  focused app / self-dismissing.
- Multi-monitor positioning rewritten to use native AppKit coordinates
  (`NSEvent.mouseLocation` + `NSScreen`) instead of scaled tray-event coords.

## [0.1.0] - 2026-07-06

### Added
- Initial release: macOS menubar prayer-times app for Malaysia.
- Next-prayer countdown in the menu bar; popover with the day's full schedule,
  current waqt highlight, Hijri date, and qibla bearing.
- Location via IP geolocation → JAKIM zone, with manual zone override.
- Offline monthly cache (Waktu Solat API primary, JAKIM e-Solat for qibla).
- Nocturnal teal + gold Islamic design with bilingual Malay/Arabic names.
