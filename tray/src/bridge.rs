use crate::{config, spotify, wow};
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ZoneEvent {
    zone: String,
    subzone: String,
    instance: String,
    instance_name: String,
    combat: bool,
    why: String,
}

fn kv(line: &str, key: &str) -> String {
    let pat = format!("{key}=\"");
    let Some(start) = line.find(&pat) else {
        return String::new();
    };
    let rest = &line[start + pat.len()..];
    let Some(end) = rest.find('"') else {
        return String::new();
    };
    rest[..end].to_string()
}

fn parse_line(line: &str) -> Option<ZoneEvent> {
    if !line.contains("[AZERADIO]") {
        return None;
    }
    Some(ZoneEvent {
        zone: kv(line, "zone"),
        subzone: kv(line, "subzone"),
        instance: kv(line, "instance"),
        instance_name: kv(line, "instanceName"),
        combat: kv(line, "combat") == "1",
        why: kv(line, "why"),
    })
}

/// Most specific match wins: subzone, then zone, then instance, then fallback.
/// Combat always wins while you are in combat.
fn pick_playlist<'a>(ev: &ZoneEvent, m: &'a config::Mappings) -> Option<&'a str> {
    if ev.combat {
        if let Some(c) = &m.combat_playlist {
            return Some(c);
        }
    }
    if !ev.subzone.is_empty() {
        if let Some(u) = m.subzones.get(&ev.subzone) {
            return Some(u);
        }
    }
    if !ev.zone.is_empty() {
        if let Some(u) = m.zones.get(&ev.zone) {
            return Some(u);
        }
    }
    if !ev.instance_name.is_empty() {
        if let Some(u) = m.instances.get(&ev.instance_name) {
            return Some(u);
        }
    }
    m.fallback_playlist.as_deref()
}

/// What the Settings status line shows.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct BridgeStatus {
    pub logs_watched: usize,
    pub last_event: String,
    pub last_error: String,
}

static RECENT: std::sync::OnceLock<std::sync::Mutex<Vec<ZoneEvent>>> =
    std::sync::OnceLock::new();
static STATUS: std::sync::OnceLock<std::sync::Mutex<BridgeStatus>> = std::sync::OnceLock::new();

fn recent() -> &'static std::sync::Mutex<Vec<ZoneEvent>> {
    RECENT.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

fn status() -> &'static std::sync::Mutex<BridgeStatus> {
    STATUS.get_or_init(|| std::sync::Mutex::new(BridgeStatus::default()))
}

/// Zones the game has actually announced, newest first (max 20).
/// Powers the "recently seen" shortcuts in Settings.
pub fn recent_zones() -> Vec<ZoneEvent> {
    recent().lock().map(|v| v.clone()).unwrap_or_default()
}

pub fn bridge_status() -> BridgeStatus {
    status().lock().map(|s| s.clone()).unwrap_or_default()
}

fn event_label(ev: &ZoneEvent) -> String {
    let mut name = if !ev.subzone.is_empty() && ev.subzone != ev.zone {
        format!("{} ({})", ev.subzone, ev.zone)
    } else if !ev.zone.is_empty() {
        ev.zone.clone()
    } else if !ev.instance_name.is_empty() {
        format!("instance: {}", ev.instance_name)
    } else {
        "unknown".to_string()
    };
    if ev.combat {
        name = format!("[combat] {name}");
    }
    name
}

fn note_event(ev: &ZoneEvent) {
    if let Ok(mut r) = recent().lock() {
        r.retain(|x| x.zone != ev.zone || x.subzone != ev.subzone || x.instance_name != ev.instance_name);
        r.insert(0, ev.clone());
        r.truncate(20);
    }
    if let Ok(mut s) = status().lock() {
        s.last_event = event_label(ev);
        journal("info", event_label(ev));
        s.last_error.clear();
    }
}

fn note_logs(n: usize) {
    if let Ok(mut s) = status().lock() {
        s.logs_watched = n;
    }
}

fn note_error(e: &str) {
    if let Ok(mut s) = status().lock() {
        s.last_error = e.to_string();
        journal("err", e.to_string());
    }
}

/// One row of the Settings activity feed. `ts` is unix seconds; the UI
/// renders it in local time.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEntry {
    pub ts: u64,
    pub kind: String,
    pub text: String,
}

static JOURNAL: std::sync::OnceLock<std::sync::Mutex<VecDeque<LogEntry>>> =
    std::sync::OnceLock::new();

fn journal_store() -> &'static std::sync::Mutex<VecDeque<LogEntry>> {
    JOURNAL.get_or_init(|| std::sync::Mutex::new(VecDeque::new()))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn journal(kind: &str, text: String) {
    if let Ok(mut j) = journal_store().lock() {
        j.push_back(LogEntry {
            ts: now_unix(),
            kind: kind.to_string(),
            text,
        });
        while j.len() > 100 {
            j.pop_front();
        }
    }
}

pub fn activity() -> Vec<LogEntry> {
    journal_store()
        .lock()
        .map(|j| j.iter().cloned().collect())
        .unwrap_or_default()
}

/// Short label for a watched log: the flavor folder name without
/// underscores ("retail"), or "custom" for the env-var override.
fn log_label(p: &Path) -> String {
    p.parent()
        .and_then(|d| d.file_name())
        .map(|n| n.to_string_lossy().trim_matches('_').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "custom".to_string())
}

fn handle(
    ev: &ZoneEvent,
    auth: &mut config::SpotifyAuth,
    m: &config::Mappings,
    current: &mut String,
) -> Result<(), String> {
    let target = pick_playlist(ev, m).unwrap_or("").to_string();
    if target.is_empty() || target == *current {
        return Ok(());
    }
    spotify::ensure_token(auth)?;
    spotify::play_context(auth, &target, m.device_id.as_deref())?;
    journal("ok", format!("{} → {}", event_label(ev), target));
    *current = target;
    Ok(())
}

fn watch() -> Result<(), String> {
    config::ensure_sample_files();
    let mut auth = config::load_auth();
    // Read offset per log file. A file seen for the first time starts at
    // its end, so enabling chat logging mid-run doesn't replay history.
    let mut pos: HashMap<PathBuf, u64> = HashMap::new();
    let mut current = String::new();
    let mut prev_logs: Option<Vec<PathBuf>> = None;
    let mut ticks: u64 = 0;

    loop {
        std::thread::sleep(Duration::from_millis(500));
        ticks += 1;
        // Re-resolved every tick so Settings edits and newly enabled chat
        // logging take effect without a restart.
        let logs = wow::active_logs(&config::load_wow());
        note_logs(logs.len());
        if prev_logs.as_ref() != Some(&logs) {
            if logs.is_empty() {
                journal("info", "No chat logs found".to_string());
            } else {
                let names: Vec<String> = logs.iter().map(|l| log_label(l)).collect();
                journal("info", format!("Watching: {}", names.join(", ")));
            }
            prev_logs = Some(logs.clone());
        }
        if logs.is_empty() {
            // Remind at most every ~30s so the console doesn't drown.
            if ticks % 60 == 1 {
                eprintln!("[azeradio] no WoW chat logs found — set install paths in Settings, or enable chat logging in game with /console chatLog 1");
            }
            continue;
        }
        if auth.access_token.is_empty() && auth.refresh_token.is_empty() {
            // First run: log in now so the browser opens at startup, not mid-pull.
            spotify::oauth_flow(&mut auth)?;
        }

        for log in &logs {
            let len = std::fs::metadata(log).map(|m| m.len()).unwrap_or(0);
            let p = pos.entry(log.clone()).or_insert(len);
            if len < *p {
                *p = 0; // log was rotated; start over
            }
            if len <= *p {
                continue;
            }
            let mut f = File::open(log).map_err(|e| e.to_string())?;
            f.seek(SeekFrom::Start(*p)).map_err(|e| e.to_string())?;
            let mut buf = String::new();
            f.read_to_string(&mut buf).map_err(|e| e.to_string())?;
            *p = len;
            for line in buf.lines() {
                let Some(ev) = parse_line(line) else { continue };
                note_event(&ev);
                // Reloaded per event so editing mappings.json takes effect
                // on the next zone change. No restart needed.
                let mappings = config::load_mappings();
                if let Err(e) = handle(&ev, &mut auth, &mappings, &mut current) {
                    eprintln!("[azeradio] {e}");
                    note_error(&e);
                }
            }
        }
        // Forget deleted logs so a recreated file starts at its end.
        pos.retain(|k, _| logs.contains(k));
    }
}

/// Runs forever on its own thread; restarts the watcher if it fails.
pub fn run() {
    loop {
        if let Err(e) = watch() {
            eprintln!("[azeradio] watcher stopped: {e}");
            note_error(&format!("watcher stopped: {e}"));
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mappings() -> config::Mappings {
        config::Mappings {
            zones: HashMap::from([("Orgrimmar".into(), "spotify:playlist:zone".into())]),
            subzones: HashMap::from([("Valdrakken".into(), "spotify:playlist:sub".into())]),
            instances: HashMap::from([("Amirdrassil".into(), "spotify:playlist:inst".into())]),
            combat_playlist: Some("spotify:playlist:combat".into()),
            fallback_playlist: Some("spotify:playlist:fallback".into()),
            device_id: None,
        }
    }

    #[test]
    fn parses_line_with_instance_name() {
        let ev = parse_line("[AZERADIO] zone=\"X\" subzone=\"Y\" instance=\"raid\" instanceName=\"Amirdrassil\" instanceID=\"2\" combat=\"0\" why=\"zone\"").unwrap();
        assert_eq!(ev.instance_name, "Amirdrassil");
        assert_eq!(ev.zone, "X");
        assert!(!ev.combat);
    }

    #[test]
    fn parses_old_line_without_instance_name() {
        let ev = parse_line("[AZERADIO] zone=\"X\" subzone=\"\" instance=\"raid\" instanceID=\"2\" combat=\"1\" why=\"combat_start\"").unwrap();
        assert_eq!(ev.instance_name, "");
        assert!(ev.combat);
    }

    #[test]
    fn ignores_non_azeradio_lines() {
        assert!(parse_line("just chat").is_none());
    }

    #[test]
    fn most_specific_match_wins() {
        let m = mappings();
        let ev = ZoneEvent { zone: "Orgrimmar".into(), subzone: "Valdrakken".into(), ..Default::default() };
        assert_eq!(pick_playlist(&ev, &m), Some("spotify:playlist:sub"));
        let ev = ZoneEvent { instance_name: "Amirdrassil".into(), ..Default::default() };
        assert_eq!(pick_playlist(&ev, &m), Some("spotify:playlist:inst"));
        let ev = ZoneEvent { zone: "Nowhere".into(), ..Default::default() };
        assert_eq!(pick_playlist(&ev, &m), Some("spotify:playlist:fallback"));
        let ev = ZoneEvent { zone: "Orgrimmar".into(), subzone: "Valdrakken".into(), combat: true, ..Default::default() };
        assert_eq!(pick_playlist(&ev, &m), Some("spotify:playlist:combat"));
    }
}