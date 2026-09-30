use crate::{config, spotify};
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

fn find_log() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("AZERADIO_WOW_LOG") {
        return Ok(p.into());
    }
    let mut candidates: Vec<String> = Vec::new();
    #[cfg(target_os = "windows")]
    for drive in ["C:", "D:"] {
        for base in [
            "Program Files\\World of Warcraft",
            "Program Files (x86)\\World of Warcraft",
        ] {
            for flavor in ["_retail_", "_classic_beta_"] {
                candidates.push(format!(
                    "{drive}\\{base}\\{flavor}\\Logs\\WoWChatLog.txt"
                ));
            }
        }
    }
    #[cfg(target_os = "macos")]
    for flavor in ["_retail_", "_classic_beta_"] {
        candidates.push(format!(
            "/Applications/World of Warcraft/{flavor}/Logs/WoWChatLog.txt"
        ));
    }
    for c in candidates {
        let p = PathBuf::from(&c);
        if p.exists() {
            return Ok(p);
        }
    }
    Err("WoW chat log not found. Enable chat logging in game or set AZERADIO_WOW_LOG.".into())
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
    let log = find_log()?;
    let mut auth = config::load_auth();
    if auth.access_token.is_empty() && auth.refresh_token.is_empty() {
        // First run: log in now so the browser opens at startup, not mid-pull.
        spotify::oauth_flow(&mut auth)?;
    }
    let mappings = config::load_mappings();

    // Start at the end of the log so old lines do not replay.
    let mut pos = std::fs::metadata(&log).map(|m| m.len()).unwrap_or(0);
    let mut current = String::new();

    loop {
        std::thread::sleep(Duration::from_millis(500));
        let len = std::fs::metadata(&log).map(|m| m.len()).unwrap_or(0);
        if len < pos {
            pos = 0; // log was rotated; start over
        }
        if len <= pos {
            continue;
        }
        let mut f = File::open(&log).map_err(|e| e.to_string())?;
        f.seek(SeekFrom::Start(pos)).map_err(|e| e.to_string())?;
        let mut buf = String::new();
        f.read_to_string(&mut buf).map_err(|e| e.to_string())?;
        pos = len;
        for line in buf.lines() {
            let Some(ev) = parse_line(line) else { continue };
            if let Err(e) = handle(&ev, &mut auth, &mappings, &mut current) {
                eprintln!("[azeradio] {e}");
            }
        }
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
