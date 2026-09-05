mod capture;
mod commands;
mod history;
mod i18n;
mod onboarding;
mod settings;
mod shortcuts;
mod tray;
mod windows;

use tauri::Manager;

use commands::AppState;
use history::History;
use settings::{EditorPrefsStore, RecorderPrefsStore, SettingsStore};

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
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            #[cfg(target_os = "macos")]
            {
                use objc2::AnyThread;
                use objc2_app_kit::{NSApplication, NSImage};
                use objc2_foundation::{MainThreadMarker, NSData};
                let mtm = MainThreadMarker::new().expect("setup runs on the main thread");
                let data = NSData::with_bytes(include_bytes!("../icons/128x128@2x.png"));
                if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
                    unsafe {
                        NSApplication::sharedApplication(mtm)
                            .setApplicationIconImage(Some(&image));
                    }
                }
            }

            let data_dir = app.path().app_data_dir()?;
            app.manage(AppState::new(
                History::new(data_dir.join("captures"))?,
                SettingsStore::load(data_dir.join("settings.json")),
                EditorPrefsStore::load(data_dir.join("editor_prefs.json")),
                RecorderPrefsStore::load(data_dir.join("recorder_prefs.json")),
            ));

            let language = app.state::<AppState>().settings.get().language;
            i18n::set_language(i18n::resolve(&language));

            tray::setup(app.handle())?;
            shortcuts::setup(app.handle());

            let welcome_marker = data_dir.join("welcome_seen");
            if !welcome_marker.exists() {
                let _ = std::fs::write(&welcome_marker, b"");
                if let Err(err) = windows::open_welcome(app.handle().clone()) {
                    eprintln!("failed to open welcome window: {err}");
                }
            }

            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_title(&i18n::t("window.settings"));
            }

            windows::announce_pending_update(app.handle(), &data_dir);

            #[cfg(not(debug_assertions))]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    loop {
                        windows::check_for_updates(&handle, true);
                        tokio::time::sleep(std::time::Duration::from_secs(60 * 60 * 24))
                            .await;
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_captures,
            commands::copy_capture,
            commands::restore_capture,
            commands::save_capture_to,
            commands::reveal_capture,
            commands::open_editor,
            commands::read_capture_bytes,
            commands::editor_target,
            commands::export_png,
            commands::get_settings,
            commands::set_settings,
            commands::set_shortcut,
            commands::resolved_language,
            commands::get_editor_prefs,
            commands::set_editor_prefs,
            commands::save_capture_to_desktop,
            commands::set_overlay_drag_active,
            commands::set_overlay_panels,
            windows::open_history,
            windows::open_welcome,
            onboarding::open_system_shortcut_settings,
            onboarding::macos_screenshot_hotkeys_owned,
            onboarding::apply_macos_screenshot_shortcuts,
            commands::timer_duration,
            commands::timed_capture_fire,
            commands::run_scrolling_capture,
            commands::stop_scrolling_capture,
            commands::start_recording,
            commands::stop_recording,
            commands::recorder_prefs,
            commands::is_recording,
            commands::open_capture,
        ])
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
