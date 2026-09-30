use crate::{config, spotify, wow};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Default)]
struct ZoneEvent {
    zone: String,
    subzone: String,
    instance: String,
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
    if !ev.instance.is_empty() && ev.instance != "none" {
        if let Some(u) = m.instances.get(&ev.instance) {
            return Some(u);
        }
    }
    m.fallback_playlist.as_deref()
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
    let mut ticks: u64 = 0;

    loop {
        std::thread::sleep(Duration::from_millis(500));
        ticks += 1;
        // Re-resolved every tick so Settings edits and newly enabled chat
        // logging take effect without a restart.
        let logs = wow::active_logs(&config::load_wow());
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
                // Reloaded per event so editing mappings.json takes effect
                // on the next zone change. No restart needed.
                let mappings = config::load_mappings();
                if let Err(e) = handle(&ev, &mut auth, &mappings, &mut current) {
                    eprintln!("[azeradio] {e}");
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
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}
