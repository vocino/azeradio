mod bridge;
mod config;
mod spotify;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};

fn main() {
    std::thread::spawn(bridge::run);

    tauri::Builder::default()
        .setup(|app| {
            let status =
                MenuItem::with_id(app, "status", "Azeradio is running", false, None::<&str>)?;
            let open_config =
                MenuItem::with_id(app, "open_config", "Open config folder", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&status, &sep, &open_config, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Azeradio")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open_config" => {
                        let dir = config::config_dir();
                        std::fs::create_dir_all(&dir).ok();
                        let _ = open::that(&dir);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("azeradio failed to start");
}
