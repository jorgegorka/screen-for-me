mod capture;
mod commands;
mod history;
mod settings;
mod shortcuts;
mod tray;
mod windows;

use tauri::Manager;

use commands::AppState;
use history::History;
use settings::{EditorPrefsStore, SettingsStore};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Menu-bar app: no Dock icon on macOS.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let data_dir = app.path().app_data_dir()?;
            app.manage(AppState {
                history: History::new(data_dir.join("captures"))?,
                settings: SettingsStore::load(data_dir.join("settings.json")),
                editor_prefs: EditorPrefsStore::load(data_dir.join("editor_prefs.json")),
                editor_target: std::sync::Mutex::new(None),
                timer_seconds: std::sync::Mutex::new(5),
                last_capture_mode: std::sync::Mutex::new(crate::capture::CaptureMode::Fullscreen),
                scroll_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                scroll_running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            commands::get_capture,
            commands::read_capture_bytes,
            commands::editor_target,
            commands::export_png,
            commands::get_settings,
            commands::set_settings,
            commands::get_editor_prefs,
            commands::set_editor_prefs,
            commands::save_capture_to_desktop,
            windows::open_history,
            commands::timer_duration,
            commands::timed_capture_fire,
            commands::run_scrolling_capture,
            commands::stop_scrolling_capture,
        ])
        // The main window doubles as the Settings window: closing it hides it
        // so the tray can re-show it without recreating.
        // The main (Settings) and editor windows hide instead of being
        // destroyed, so the tray/overlay can re-show them and the editor's
        // webview stays warm between annotations.
        .on_window_event(|window, event| {
            if matches!(window.label(), "main" | "editor" | "history") {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
