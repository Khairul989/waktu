//! Pure prayer-time computation core: row list + next-fard-prayer logic.
//!
//! No I/O here on purpose — everything is a fn of `(today, tomorrow_fajr, now)`
//! so it's trivial to unit test with fixed epoch fixtures.

use crate::models::{DayPrayer, NextPrayer, PrayerRow};

/// The 7 marks in display order: (key, Malay name, Arabic name, epoch seconds, is_fard).
/// Imsak/Syuruk are markers only (never "active"); the 5 fard prayers drive the countdown.
fn marks(today: &DayPrayer) -> [(&'static str, &'static str, &'static str, i64, bool); 7] {
    [
        ("imsak", "Imsak", "إمساك", today.imsak, false),
        ("subuh", "Subuh", "الفجر", today.fajr, true),
        ("syuruk", "Syuruk", "الشروق", today.syuruk, false),
        ("zohor", "Zohor", "الظهر", today.dhuhr, true),
        ("asar", "Asar", "العصر", today.asr, true),
        ("maghrib", "Maghrib", "المغرب", today.maghrib, true),
        ("isyak", "Isyak", "العشاء", today.isha, true),
    ]
}

/// Builds the 7-row prayer list with `passed`/`is_active` flags.
///
/// `is_active` = the fard window (this mark's time until the next mark's time,
/// or `tomorrow_fajr` for Isyak) contains `now`. Non-fard marks (Imsak, Syuruk)
/// are never active — they're reference points, not prayer windows.
pub fn build_prayer_rows(today: &DayPrayer, tomorrow_fajr: i64, now: i64) -> Vec<PrayerRow> {
    let m = marks(today);
    m.iter()
        .enumerate()
        .map(|(i, (key, name, arabic, at_sec, is_fard))| {
            let window_end = m.get(i + 1).map(|next| next.3).unwrap_or(tomorrow_fajr);
            let is_active = *is_fard && *at_sec <= now && now < window_end;
            PrayerRow {
                key: (*key).to_string(),
                name: (*name).to_string(),
                arabic: (*arabic).to_string(),
                at_ms: at_sec * 1000,
                passed: now >= *at_sec,
                is_active,
            }
        })
        .collect()
}

/// The next fard prayer after `now`; rolls to tomorrow's Subuh after Isyak.
pub fn next_prayer(today: &DayPrayer, tomorrow_fajr: i64, now: i64) -> NextPrayer {
    let fard = [
        ("Subuh", today.fajr),
        ("Zohor", today.dhuhr),
        ("Asar", today.asr),
        ("Maghrib", today.maghrib),
        ("Isyak", today.isha),
    ];
    for (name, at) in fard {
        if at > now {
            return NextPrayer {
                name: name.to_string(),
                at_ms: at * 1000,
            };
        }
    }
    NextPrayer {
        name: "Subuh".to_string(),
        at_ms: tomorrow_fajr * 1000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture day: 2026-07-06, Asia/Kuala_Lumpur (UTC+8). Computed independently
    // in Python (`datetime(...).timestamp()`) so the expected epochs aren't
    // derived from the code under test.
    fn fixture_day() -> DayPrayer {
        DayPrayer {
            day: 6,
            hijri: "1448-01-21".to_string(),
            imsak: 1783288200,   // 05:50
            fajr: 1783288800,    // 06:00
            syuruk: 1783293300,  // 07:15
            dhuha: 1783294800,   // 07:40
            dhuhr: 1783315380,   // 13:23
            asr: 1783327500,     // 16:45
            maghrib: 1783337040, // 19:24
            isha: 1783341300,    // 20:35
        }
    }

    const TOMORROW_FAJR: i64 = 1783375260; // 2026-07-07 06:01 local

    #[test]
    fn mid_morning_next_is_zohor() {
        let day = fixture_day();
        let now = 1783303200; // 10:00 local — between Syuruk and Zohor
        let next = next_prayer(&day, TOMORROW_FAJR, now);
        assert_eq!(next.name, "Zohor");
        assert_eq!(next.at_ms, day.dhuhr * 1000);
    }

    #[test]
    fn just_before_asar_next_is_asar_and_not_passed() {
        let day = fixture_day();
        let now = 1783327200; // 16:40 local — 5 min before Asar
        let next = next_prayer(&day, TOMORROW_FAJR, now);
        assert_eq!(next.name, "Asar");
        assert_eq!(next.at_ms, day.asr * 1000);

        let rows = build_prayer_rows(&day, TOMORROW_FAJR, now);
        let asar = rows.iter().find(|r| r.key == "asar").unwrap();
        assert!(!asar.passed, "Asar should not be marked passed yet");
        assert!(!asar.is_active, "Asar window hasn't started yet");
    }

    #[test]
    fn after_isyak_next_is_tomorrow_subuh() {
        let day = fixture_day();
        let now = 1783344600; // 21:30 local — after Isyak
        let next = next_prayer(&day, TOMORROW_FAJR, now);
        assert_eq!(next.name, "Subuh");
        assert_eq!(next.at_ms, TOMORROW_FAJR * 1000);

        let rows = build_prayer_rows(&day, TOMORROW_FAJR, now);
        let isyak = rows.iter().find(|r| r.key == "isyak").unwrap();
        assert!(isyak.passed);
        assert!(isyak.is_active, "still within Isyak's window until tomorrow's Subuh");
    }

    #[test]
    fn active_waqt_detection_during_asar() {
        let day = fixture_day();
        let now = 1783328400; // 17:00 local — inside the Asar window
        let rows = build_prayer_rows(&day, TOMORROW_FAJR, now);

        let asar = rows.iter().find(|r| r.key == "asar").unwrap();
        assert!(asar.passed);
        assert!(asar.is_active);

        // Every other row must not be active — only one waqt is active at a time.
        for row in rows.iter().filter(|r| r.key != "asar") {
            assert!(!row.is_active, "{} should not be active", row.key);
        }
    }
}
