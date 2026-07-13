use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

use crate::capture::CaptureMode;
use crate::commands::trigger_capture;
use crate::shortcuts;
use crate::windows;

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let area = MenuItem::with_id(app, "capture_area", "Capture Area", true, Some(shortcuts::ACCEL_AREA))?;
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
    let timer_3 = MenuItem::with_id(app, "timer_3", "3 seconds", true, None::<&str>)?;
    let timer_5 = MenuItem::with_id(app, "timer_5", "5 seconds", true, None::<&str>)?;
    let timer_10 = MenuItem::with_id(app, "timer_10", "10 seconds", true, None::<&str>)?;
    let self_timer = Submenu::with_items(app, "Self-Timer", true, &[&timer_3, &timer_5, &timer_10])?;
    let history = MenuItem::with_id(app, "history", "Capture History…", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "About Screen for me…", true, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", "Check for Updates…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
    let quit = MenuItem::with_id(app, "quit", "Quit Screen for me", true, Some("CmdOrCtrl+Q"))?;

    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let sep4 = PredefinedMenuItem::separator(app)?;
    let mut items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        vec![&area, &window, &fullscreen, &sep1];
    items.push(&self_timer);
    items.extend_from_slice(&[
        &sep2, &history, &sep3, &about, &updates, &sep4, &settings, &quit,
    ]);
    let menu = Menu::with_items(app, &items)?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().expect("bundle has an icon").clone())
        .icon_as_template(true)
        .tooltip("Screen for me")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture_area" => trigger_capture(app, CaptureMode::Area),
            "capture_window" => trigger_capture(app, CaptureMode::Window),
            "capture_fullscreen" => trigger_capture(app, CaptureMode::Fullscreen),
            "timer_3" => crate::commands::start_timed_capture(app, 3),
            "timer_5" => crate::commands::start_timed_capture(app, 5),
            "timer_10" => crate::commands::start_timed_capture(app, 10),
            "history" => {
                if let Err(err) = windows::open_history(app.clone()) {
                    eprintln!("failed to open history: {err}");
                }
            }
            "about" => windows::show_about(app),
            "updates" => windows::check_for_updates(app),
            "settings" => windows::open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
