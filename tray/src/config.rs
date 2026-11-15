use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Zone/subzone/instance name -> Spotify playlist URI, e.g.
/// "The Waking Shores" -> "spotify:playlist:37i9dQZF1DX3..."
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Mappings {
    #[serde(default)]
    pub zones: HashMap<String, String>,
    #[serde(default)]
    pub subzones: HashMap<String, String>,
    #[serde(default)]
    pub instances: HashMap<String, String>,
    #[serde(default)]
    pub combat_playlist: Option<String>,
    #[serde(default)]
    pub fallback_playlist: Option<String>,
    /// Optional: pin playback to one Spotify Connect device.
    /// Paste the device id (Advanced in Settings); empty means "wherever Spotify plays".
    #[serde(default)]
    pub device_id: Option<String>,
}

/// Bring your own key: the user pastes their own Spotify client ID here.
/// No secret — the app logs in with PKCE (see spotify.rs).
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SpotifyAuth {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub expires_at: u64,
}

/// WoW install directory: the Forever client folder itself, e.g.
/// `D:\Blizzard\World of Warcraft\_forever_`. Empty string = auto-detect.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WowPaths {
    #[serde(default)]
    pub forever: String,
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("azeradio")
}

fn read_json<T: serde::de::DeserializeOwned + Default>(name: &str) -> T {
    let path = config_dir().join(name);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn load_mappings() -> Mappings {
    read_json("mappings.json")
}

pub fn load_auth() -> SpotifyAuth {
    read_json("spotify.json")
}

pub fn load_wow() -> WowPaths {
    read_json("wow.json")
}

pub fn save_wow(wow: &WowPaths) {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).ok();
    if let Ok(s) = serde_json::to_string_pretty(wow) {
        let _ = std::fs::write(dir.join("wow.json"), s);
    }
}

pub fn save_mappings(m: &Mappings) {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).ok();
    if let Ok(s) = serde_json::to_string_pretty(m) {
        let _ = std::fs::write(dir.join("mappings.json"), s);
    }
}

pub fn save_auth(auth: &SpotifyAuth) {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).ok();
    if let Ok(s) = serde_json::to_string_pretty(auth) {
        let _ = std::fs::write(dir.join("spotify.json"), s);
    }
}

/// Writes starter config files on first run so there is something to edit.
pub fn ensure_sample_files() {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).ok();

    let mappings = dir.join("mappings.json");
    if !mappings.exists() {
        // Ships with the Azeroth soundtrack library mapped, so zone
        // switching works out of the box. Users can remap any zone to
        // their own playlists. Library: https://vocino.github.io/azeradio/
        let lib: Mappings = parse_library(BUILTIN_LIBRARY).unwrap_or_default();
        save_mappings(&lib);
    }

    let auth = dir.join("spotify.json");
    if !auth.exists() {
        let sample = SpotifyAuth::default();
        save_auth(&sample);
    }
}

/// Remote source of truth for the zone library, so new playlists reach
/// existing installs without an app release. Same file that is baked in
/// below, served from main: edit `tray/library.json`, push, done.
pub const LIBRARY_URL: &str =
    "https://raw.githubusercontent.com/vocino/azeradio/refs/heads/main/tray/library.json";

/// Baked-in copy of the library for first run and offline fallback.
const BUILTIN_LIBRARY: &str = include_str!("../library.json");

fn parse_library(s: &str) -> Option<Mappings> {
    serde_json::from_str(s).ok()
}

fn fetch_library() -> Option<Mappings> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;
    let body = client.get(LIBRARY_URL).send().ok()?.text().ok()?;
    parse_library(&body)
}

/// Additive merge: fills in library zones the user doesn't have, never
/// overwrites a user's own rows. Combat/fallback stay user-controlled.
/// Returns the number of rows added.
pub fn merge_library(m: &mut Mappings, lib: &Mappings) -> usize {
    let mut added = 0;
    for (table, lib_table) in [
        (&mut m.zones, &lib.zones),
        (&mut m.subzones, &lib.subzones),
        (&mut m.instances, &lib.instances),
    ] {
        for (k, v) in lib_table {
            if !table.contains_key(k) {
                table.insert(k.clone(), v.clone());
                added += 1;
            }
        }
    }
    added
}

/// Seed from the baked-in library on first run; afterwards pull the remote
/// library and merge new zones in. Never overwrites user rows. Safe to call
/// on every startup. Note: deleting a library row is not sticky — the next
/// sync re-adds it. To silence a zone, point it at the fallback instead.
pub fn sync_library() {
    let mut m = load_mappings();
    let lib = fetch_library().or_else(|| parse_library(BUILTIN_LIBRARY));
    if let Some(lib) = lib {
        if merge_library(&mut m, &lib) > 0 {
            save_mappings(&m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> Mappings {
        parse_library(BUILTIN_LIBRARY).expect("bundled library.json must parse")
    }

    #[test]
    fn builtin_library_has_stormwind() {
        assert_eq!(
            lib().zones.get("Stormwind"),
            Some(&"spotify:playlist:0jvepuHBLsYUE9uDFDidmC".to_string())
        );
    }

    #[test]
    fn merge_adds_missing_and_keeps_user_rows() {
        let mut m = Mappings::default();
        m.zones.insert(
            "Orgrimmar".to_string(),
            "spotify:playlist:user-choice".to_string(),
        );
        let added = merge_library(&mut m, &lib());
        assert!(added > 0);
        // User row untouched.
        assert_eq!(
            m.zones.get("Orgrimmar"),
            Some(&"spotify:playlist:user-choice".to_string())
        );
        // Missing library row filled in.
        assert_eq!(
            m.zones.get("Stormwind"),
            Some(&"spotify:playlist:0jvepuHBLsYUE9uDFDidmC".to_string())
        );
        // Second merge is a no-op.
        assert_eq!(merge_library(&mut m, &lib()), 0);
    }

    #[test]
    fn merge_leaves_combat_and_fallback_alone() {
        let mut m = Mappings::default();
        let mut l = lib();
        l.combat_playlist = Some("spotify:playlist:xyz".to_string());
        l.fallback_playlist = Some("spotify:playlist:xyz".to_string());
        merge_library(&mut m, &l);
        assert_eq!(m.combat_playlist, None);
        assert_eq!(m.fallback_playlist, None);
    }
}
