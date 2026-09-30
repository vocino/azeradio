use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Zone/subzone/instance name -> Spotify playlist URI, e.g.
/// "The Waking Shores" -> "spotify:playlist:37i9dQZF1DX3..."
#[derive(Debug, Deserialize, Default)]
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
    /// Optional: pin playback to one device. Find yours via the
    /// "devices" section in docs/SETUP.md.
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SpotifyAuth {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub expires_at: u64,
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
        let sample = serde_json::json!({
            "zones": {
                "Orgrimmar": "spotify:playlist:PASTE_PLAYLIST_URI_HERE"
            },
            "subzones": {},
            "instances": {},
            "combat_playlist": "spotify:playlist:PASTE_COMBAT_PLAYLIST_HERE",
            "fallback_playlist": "spotify:playlist:PASTE_DEFAULT_PLAYLIST_HERE",
            "device_id": null
        });
        let _ = std::fs::write(
            &mappings,
            serde_json::to_string_pretty(&sample).unwrap(),
        );
    }

    let auth = dir.join("spotify.json");
    if !auth.exists() {
        let sample = SpotifyAuth::default();
        save_auth(&sample);
    }
}
