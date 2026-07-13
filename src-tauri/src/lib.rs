mod capture;
mod commands;
mod history;
mod shortcuts;
mod tray;

use tauri::Manager;

use commands::AppState;
use history::History;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Menu-bar app: no Dock icon on macOS.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let captures_dir = app.path().app_data_dir()?.join("captures");
            app.manage(AppState {
                history: History::new(captures_dir)?,
            });

            tray::setup(app.handle())?;
            shortcuts::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::capture_screen,
            commands::list_captures,
            commands::delete_capture,
            commands::copy_capture,
            commands::save_capture_to,
            commands::reveal_capture,
            commands::open_editor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
