mod capture;
mod commands;
mod history;
mod i18n;
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
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Menu-bar app: no Dock icon on macOS.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Give NSAlert dialogs (About, updater) the app icon even in dev,
            // where the bare binary has no bundle icon to fall back on.
            #[cfg(target_os = "macos")]
            {
                use objc2::AnyThread;
                use objc2_app_kit::{NSApplication, NSImage};
                use objc2_foundation::{MainThreadMarker, NSData};
                let mtm = MainThreadMarker::new().expect("setup runs on the main thread");
                let data = NSData::with_bytes(include_bytes!("../icons/128x128@2x.png"));
                if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
                    // Safety: valid NSImage, called on the main thread (mtm).
                    unsafe {
                        NSApplication::sharedApplication(mtm)
                            .setApplicationIconImage(Some(&image));
                    }
                }
            }

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

            // Resolve the UI language before anything user-visible is built.
            let language = app.state::<AppState>().settings.get().language;
            i18n::set_language(i18n::resolve(&language));

            tray::setup(app.handle())?;
            shortcuts::setup(app.handle())?;

            // Config-declared windows carry the English titles from
            // tauri.conf.json; retitle the visible one for the active language.
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_title(&i18n::t("window.settings"));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_captures,
            commands::copy_capture,
            commands::save_capture_to,
            commands::reveal_capture,
            commands::open_editor,
            commands::read_capture_bytes,
            commands::editor_target,
            commands::export_png,
            commands::get_settings,
            commands::set_settings,
            commands::resolved_language,
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
            if windows::HIDE_ON_CLOSE.contains(&window.label()) {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
