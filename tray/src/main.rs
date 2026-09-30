mod bridge;
mod config;
mod spotify;
mod wow;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};

#[derive(serde::Serialize)]
struct Status {
    client_id_set: bool,
    connected: bool,
    client_id_hint: String,
}

#[tauri::command]
fn get_status() -> Status {
    let auth = config::load_auth();
    Status {
        client_id_set: !auth.client_id.is_empty(),
        connected: !auth.access_token.is_empty(),
        client_id_hint: client_hint(&auth.client_id),
    }
}

/// Paste flow: the settings window writes the ID here, the watcher thread
/// picks it up within seconds and opens the browser login on its own.
#[tauri::command]
fn save_client_id(client_id: String) -> Result<(), String> {
    let id = client_id.trim().to_string();
    if id.is_empty() {
        return Err("paste your client ID first".into());
    }
    let mut auth = config::load_auth();
    auth.client_id = id;
    config::save_auth(&auth);
    Ok(())
}

fn client_hint(id: &str) -> String {
    if id.len() >= 8 && id.is_ascii() {
        format!("{}…{}", &id[..4], &id[id.len() - 4..])
    } else {
        String::new()
    }
}

#[derive(serde::Serialize)]
struct WowConfig {
    retail: wow::FlavorState,
    classic: wow::FlavorState,
    forever: wow::FlavorState,
}

#[tauri::command]
fn get_wow() -> WowConfig {
    let w = config::load_wow();
    WowConfig {
        retail: wow::state("retail", &w.retail),
        classic: wow::state("classic", &w.classic),
        forever: wow::state("forever", &w.forever),
    }
}

/// Empty string per flavor = auto-detect. Trims, so clearing a field
/// restores detection for that flavor.
#[tauri::command]
fn save_wow(retail: String, classic: String, forever: String) {
    config::save_wow(&config::WowPaths {
        retail: retail.trim().to_string(),
        classic: classic.trim().to_string(),
        forever: forever.trim().to_string(),
    });
}

#[tauri::command]
fn open_config_folder() {
    open_config_dir();
}

#[tauri::command]
fn open_dashboard() {
    let _ = open::that("https://developer.spotify.com/dashboard");
}

#[tauri::command]
fn open_guide() {
    let _ = open::that("https://github.com/vocino/azeradio/blob/main/docs/SETUP.md");
}

fn open_config_dir() {
    let dir = config::config_dir();
    std::fs::create_dir_all(&dir).ok();
    let _ = open::that(&dir);
}

fn main() {
    std::thread::spawn(bridge::run);

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_status,
            save_client_id,
            get_wow,
            save_wow,
            open_config_folder,
            open_dashboard,
            open_guide
        ])
        .setup(|app| {
            let status =
                MenuItem::with_id(app, "status", "Azeradio is running", false, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let open_config =
                MenuItem::with_id(app, "open_config", "Open config folder", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu =
                Menu::with_items(app, &[&status, &settings, &sep, &open_config, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Azeradio")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "open_config" => open_config_dir(),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("azeradio failed to start");
}
