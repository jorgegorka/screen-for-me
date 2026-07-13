use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

use crate::capture::CaptureMode;
use crate::commands::trigger_capture;
use crate::shortcuts;

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let area = MenuItem::with_id(
        app,
        "capture_area",
        "Capture Area",
        true,
        Some(shortcuts::ACCEL_AREA),
    )?;
    let window = MenuItem::with_id(
        app,
        "capture_window",
        "Capture Window",
        true,
        Some(shortcuts::ACCEL_WINDOW),
    )?;
    let fullscreen = MenuItem::with_id(
        app,
        "capture_fullscreen",
        "Capture Fullscreen",
        true,
        Some(shortcuts::ACCEL_FULLSCREEN),
    )?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Screen for me", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &area,
            &window,
            &fullscreen,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().expect("bundle has an icon").clone())
        .icon_as_template(true)
        .tooltip("Screen for me")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture_area" => trigger_capture(app, CaptureMode::Area),
            "capture_window" => trigger_capture(app, CaptureMode::Window),
            "capture_fullscreen" => trigger_capture(app, CaptureMode::Fullscreen),
            "settings" => {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
