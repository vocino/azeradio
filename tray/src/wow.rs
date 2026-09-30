use crate::config;
use std::path::{Path, PathBuf};

/// Flavor key -> (client folder name, `.flavor.info` product id).
const FLAVORS: [(&str, &str, &str); 3] = [
    ("retail", "_retail_", "wow"),
    ("classic", "_classic_", "wow_classic"),
    ("forever", "_classic_beta_", "wow_classic_beta"),
];

pub fn flavor_meta(name: &str) -> (&'static str, &'static str) {
    FLAVORS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, d, id)| (*d, *id))
        .unwrap_or(("_retail_", "wow"))
}

/// One row of the Settings → World of Warcraft card.
#[derive(serde::Serialize)]
pub struct FlavorState {
    /// What the user saved ("" = auto-detect).
    pub manual: String,
    /// Flavor folder actually in use ("" when none found).
    pub path: String,
    /// "manual" | "auto" | "none".
    pub source: String,
    /// "ok" (chat log exists) | "no_log" (folder found, chat logging off) | "missing".
    pub status: String,
}

pub fn chat_log(flavor_dir: &Path) -> PathBuf {
    flavor_dir.join("Logs").join("WoWChatLog.txt")
}

/// `.flavor.info` is two lines (a header, then the product id).
/// The id is what distinguishes retail from classic from forever.
fn flavor_id(flavor_dir: &Path) -> String {
    std::fs::read_to_string(flavor_dir.join(".flavor.info"))
        .ok()
        .and_then(|s| {
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .last()
                .map(str::to_string)
        })
        .unwrap_or_default()
}

fn valid_flavor(flavor_dir: &Path, want_id: &str) -> bool {
    flavor_dir.is_dir() && flavor_id(flavor_dir) == want_id
}

/// Where Battle.net installs games, e.g. `D:/Blizzard`.
/// Lives in the launcher's own JSON config on both platforms.
fn bnet_install_root() -> Option<PathBuf> {
    let p = dirs::config_dir()?
        .join("Battle.net")
        .join("Battle.net.config");
    let s = std::fs::read_to_string(p).ok()?;
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    let root = v
        .get("Client")?
        .get("Install")?
        .get("DefaultInstallPath")?
        .as_str()?;
    if root.is_empty() {
        None
    } else {
        Some(PathBuf::from(root))
    }
}

/// Find a flavor folder without any user configuration:
/// Battle.net's own install root first, then the usual suspects.
/// Strict: the folder must carry the right flavor id, so a stray
/// `_retail_` from another Blizzard game never matches.
pub fn auto_detect(dir_name: &str, want_id: &str) -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(r) = bnet_install_root() {
        roots.push(r.join("World of Warcraft"));
    }
    #[cfg(target_os = "windows")]
    for drive in ["C:", "D:"] {
        for base in [
            "Program Files\\World of Warcraft",
            "Program Files (x86)\\World of Warcraft",
            "Blizzard\\World of Warcraft",
        ] {
            roots.push(PathBuf::from(format!("{drive}\\{base}")));
        }
    }
    #[cfg(target_os = "macos")]
    roots.push(PathBuf::from("/Applications/World of Warcraft"));
    roots
        .into_iter()
        .map(|r| r.join(dir_name))
        .find(|d| valid_flavor(d, want_id))
}

/// Resolve one flavor: an explicit path always wins (even a broken one,
/// so a typo shows as missing instead of silently using something else),
/// otherwise auto-detect.
pub fn resolve(name: &str, manual: &str) -> (Option<PathBuf>, &'static str) {
    let m = manual.trim();
    if !m.is_empty() {
        return (resolve_manual(name, m), "manual");
    }
    let (dir_name, want_id) = flavor_meta(name);
    match auto_detect(dir_name, want_id) {
        Some(p) => (Some(p), "auto"),
        None => (None, "none"),
    }
}

/// Manual paths accept either level: the flavor folder itself (has its own
/// `Logs`) or the install root (the `World of Warcraft` folder), in which
/// case we descend into the flavor and validate it.
fn resolve_manual(name: &str, m: &str) -> Option<PathBuf> {
    let p = PathBuf::from(m);
    if !p.is_dir() {
        return None;
    }
    if p.join("Logs").is_dir() {
        return Some(p);
    }
    let (dir_name, want_id) = flavor_meta(name);
    let f = p.join(dir_name);
    if valid_flavor(&f, want_id) {
        return Some(f);
    }
    Some(p)
}

pub fn state(name: &str, manual: &str) -> FlavorState {
    let (dir, source) = resolve(name, manual);
    let (path, status) = match dir {
        Some(d) if chat_log(&d).exists() => (d.to_string_lossy().into_owned(), "ok"),
        Some(d) => (d.to_string_lossy().into_owned(), "no_log"),
        None => (manual.trim().to_string(), "missing"),
    };
    FlavorState {
        manual: manual.trim().to_string(),
        path,
        source: source.to_string(),
        status: status.to_string(),
    }
}

/// Every chat log to watch right now. `AZERADIO_WOW_LOG` overrides
/// everything (one file, legacy escape hatch); otherwise one live log
/// per flavor, so retail, classic and forever can all be watched at once.
pub fn active_logs(cfg: &config::WowPaths) -> Vec<PathBuf> {
    if let Ok(p) = std::env::var("AZERADIO_WOW_LOG") {
        let p = PathBuf::from(p);
        return if p.exists() { vec![p] } else { Vec::new() };
    }
    let mut logs = Vec::new();
    for (name, manual) in [
        ("retail", cfg.retail.as_str()),
        ("classic", cfg.classic.as_str()),
        ("forever", cfg.forever.as_str()),
    ] {
        if let (Some(d), _) = resolve(name, manual) {
            let l = chat_log(&d);
            if l.exists() && !logs.contains(&l) {
                logs.push(l);
            }
        }
    }
    logs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wipe(p: &Path) {
        let _ = std::fs::remove_dir_all(p);
    }

    #[test]
    fn manual_flavor_dir_used_directly() {
        let base = std::env::temp_dir().join("azeradio-test-flavor");
        wipe(&base);
        std::fs::create_dir_all(base.join("Logs")).unwrap();
        let (dir, src) = resolve("retail", base.to_str().unwrap());
        assert_eq!(src, "manual");
        assert_eq!(dir, Some(base.clone()));
        wipe(&base);
    }

    #[test]
    fn manual_install_root_descends_to_flavor() {
        let root = std::env::temp_dir().join("azeradio-test-root");
        wipe(&root);
        let flav = root.join("_retail_");
        std::fs::create_dir_all(flav.join("Logs")).unwrap();
        std::fs::write(flav.join(".flavor.info"), "Product Flavor!STRING:0\nwow\n").unwrap();
        let (dir, src) = resolve("retail", root.to_str().unwrap());
        assert_eq!(src, "manual");
        assert_eq!(dir, Some(flav));
        wipe(&root);
    }

    #[test]
    fn manual_missing_path_resolves_none() {
        let (dir, src) = resolve("retail", "D:\\definitely\\not\\here\\_retail_");
        assert_eq!(src, "manual");
        assert!(dir.is_none());
    }
}