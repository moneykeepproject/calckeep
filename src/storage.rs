//! On-device persistence: calculation history (last 30), last-used inputs per
//! calculator, and the theme choice. Stored as one small JSON document:
//! - Desktop: <config dir>/CalcKeep/calckeep.json
//! - Web (WASM): browser localStorage key "calckeep"
//! - Android: the app's internal data directory
//! - iOS: the app sandbox's Documents directory
//!
//! Nothing leaves the device.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MAX_HISTORY: usize = 30;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HistoryEntry {
    pub id: u64,
    /// Calculator name (matches `Calc::name`)
    pub calc: String,
    /// Unix time in milliseconds
    pub ts: i64,
    /// Field values by field key
    pub values: HashMap<String, f64>,
    pub headline_label: String,
    pub headline_value: String,
    /// A few key inputs, e.g. "$400,000 · 20% · 6.5%"
    pub summary: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Saved {
    #[serde(default)]
    pub dark: Option<bool>,
    /// Last-used inputs per calculator name
    #[serde(default)]
    pub last: HashMap<String, HashMap<String, f64>>,
    /// Newest first, at most MAX_HISTORY entries
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

impl Saved {
    /// Insert a new entry at the top, or update an existing one (moved to the
    /// top). Drops the oldest entries beyond MAX_HISTORY.
    pub fn upsert(&mut self, entry: HistoryEntry) {
        self.history.retain(|e| e.id != entry.id);
        self.history.insert(0, entry);
        self.history.truncate(MAX_HISTORY);
    }

    pub fn next_id(&self) -> u64 {
        self.history.iter().map(|e| e.id).max().unwrap_or(0) + 1
    }
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// "Today, 11:35 PM" / "Yesterday, 9:02 AM" / "Nov 3, 11:35 PM" / "Nov 3, 2025"
pub fn when(ts: i64) -> String {
    use chrono::{Datelike, Local, TimeZone};
    let Some(t) = Local.timestamp_millis_opt(ts).single() else {
        return String::new();
    };
    let now = Local::now();
    let days = (now.date_naive() - t.date_naive()).num_days();
    let time = t.format("%-I:%M %p");
    match days {
        0 => format!("Today, {time}"),
        1 => format!("Yesterday, {time}"),
        _ if t.year() == now.year() => format!("{}, {time}", t.format("%b %-d")),
        _ => t.format("%b %-d, %Y").to_string(),
    }
}

pub fn load() -> Saved {
    read_raw()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(data: &Saved) {
    if let Ok(s) = serde_json::to_string(data) {
        write_raw(&s);
    }
}

// ------------------------------------------------------------ web

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
fn read_raw() -> Option<String> {
    local_storage()?.get_item("calckeep").ok()?
}

#[cfg(target_arch = "wasm32")]
fn write_raw(s: &str) {
    if let Some(ls) = local_storage() {
        let _ = ls.set_item("calckeep", s);
    }
}

// ------------------------------------------------------------ native

#[cfg(not(target_arch = "wasm32"))]
static DATA_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Mobile entry points call this with the app's private data directory.
#[cfg(not(target_arch = "wasm32"))]
pub fn set_data_dir(dir: std::path::PathBuf) {
    let _ = DATA_DIR.set(dir);
}

#[cfg(not(target_arch = "wasm32"))]
fn file_path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Ok(p) = std::env::var("CALCKEEP_DATA") {
        return Some(PathBuf::from(p));
    }
    let dir = if let Some(d) = DATA_DIR.get() {
        d.clone()
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var_os("APPDATA")?).join("CalcKeep")
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/CalcKeep")
    } else if cfg!(target_os = "ios") {
        PathBuf::from(std::env::var_os("HOME")?).join("Documents")
    } else {
        match std::env::var_os("XDG_CONFIG_HOME") {
            Some(x) => PathBuf::from(x).join("calckeep"),
            None => PathBuf::from(std::env::var_os("HOME")?).join(".config/calckeep"),
        }
    };
    Some(dir.join("calckeep.json"))
}

#[cfg(not(target_arch = "wasm32"))]
fn read_raw() -> Option<String> {
    std::fs::read_to_string(file_path()?).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn write_raw(s: &str) {
    let Some(path) = file_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // write to a temp file then rename, so a crash never leaves a half-written file
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, s).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_is_capped_and_newest_first() {
        let mut s = Saved::default();
        for i in 0..45 {
            let id = s.next_id();
            s.upsert(HistoryEntry {
                id,
                calc: format!("c{i}"),
                ..Default::default()
            });
        }
        assert_eq!(s.history.len(), MAX_HISTORY);
        assert_eq!(s.history[0].calc, "c44");
        assert_eq!(s.history[29].calc, "c15"); // c0..c14 were dropped
                                               // updating an old entry moves it to the top without growing the list
        let mut e = s.history[10].clone();
        e.headline_value = "x".into();
        s.upsert(e.clone());
        assert_eq!(s.history.len(), MAX_HISTORY);
        assert_eq!(s.history[0].id, e.id);
    }

    #[test]
    fn json_roundtrip_tolerates_missing_fields() {
        let s: Saved = serde_json::from_str("{}").unwrap();
        assert!(s.history.is_empty());
        let mut s2 = Saved {
            dark: Some(false),
            ..Default::default()
        };
        s2.upsert(HistoryEntry {
            id: 1,
            calc: "Loan Payment".into(),
            ts: 5,
            ..Default::default()
        });
        let back: Saved = serde_json::from_str(&serde_json::to_string(&s2).unwrap()).unwrap();
        assert_eq!(back.history[0].calc, "Loan Payment");
        assert_eq!(back.dark, Some(false));
    }
}
